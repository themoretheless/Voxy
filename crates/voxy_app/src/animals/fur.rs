use super::*;
use physics::strand::{BoxCollider, SphereCollider, Strand, StrandConfig};
use std::sync::{Arc, OnceLock};

pub const GUIDES: usize = 48;
pub const BONES: usize = 10;
pub const PALETTE: usize = BONES + GUIDES * 2;
#[derive(Clone, Copy, Debug)]
pub struct Root {
    pub position: Vec3,
    pub normal: Vec3,
    pub direction: Vec3,
    pub bone: usize,
    pub length: f32,
    pub draped: bool,
    pub clump_offset: Vec3,
}
impl Root {
    pub fn rest_point(&self, t: f32) -> Vec3 {
        if !self.draped { return self.position+self.direction*self.length*t; }
        let comb=(self.direction-self.normal*0.53).normalize_or_zero();
        self.position+self.length*(
            self.normal*(0.65*(std::f32::consts::PI*t).sin()+0.40*t)
            +comb*0.70*t-Vec3::Y*0.40*t*t)+self.clump_offset*t*t
    }
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
fn facial_region(a: voxy_model::animals::BreedAnatomy, p: Vec3) -> bool {
    let skull = a.head_length - a.muzzle_length;
    p.z > a.head_origin().z - skull * 0.1 && p.y > a.head_origin().y - skull * 0.65
}

fn build(cat: bool) -> Result<Groom, String> {
    let surface = asset::surface(cat)?;
    let anatomy = asset::profile(cat);
    let mut cdf = Vec::new();
    let mut total = 0.0;
    for face in surface.indices.chunks_exact(3) {
        if !face.iter().all(|&i| surface.fur[i as usize]) {
            continue;
        }
        let p = face.map_indices(surface);
        let area = (p[1] - p[0]).cross(p[2] - p[0]).length() * 0.5;
        if area > 1e-12 {
            let centroid = (p[0]+p[1]+p[2])/3.;
            total += area * if facial_region(anatomy,centroid) { 3.0 } else { 1.0 };
            cdf.push((total, [face[0], face[1], face[2]]));
        }
    }
    if total <= 0.0 {
        return Err("animal has no groomable surface".into());
    }
    let mut seed = if cat { 7641 } else { 5239 };
    let mut samples = Vec::new();
    let long_coat = asset::breed(cat) == voxy_model::animals::Breed::MaineCoon;
    for _ in 0..if long_coat { 255_000 } else { 85_000 } {
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
        let a = asset::profile(cat);
        let face = facial_region(a, p);
        let limb = p.y < a.shoulder_height * 0.48;
        let long = matches!(a.breed, voxy_model::animals::Breed::MaineCoon);
        let plush = matches!(
            a.breed,
            voxy_model::animals::Breed::SiberianHusky
                | voxy_model::animals::Breed::GermanShepherd
                | voxy_model::animals::Breed::BritishShorthair
        );
        let guard = random(&mut seed) > 0.68;
        let paw = p.y < a.paw_half_width * 1.5;
        let tail = p.z < -a.trunk_length * 0.48;
        let ruff = p.z > a.trunk_length * 0.28 && p.y > a.shoulder_height * 0.55;
        let belly = p.y < a.shoulder_height * 0.68;
        let length = if paw {
            0.0007
        } else if face {
            let muzzle = p.z > a.head_origin().z + a.head_length*0.75;
            if muzzle { 0.0015 } else if p.x.abs() > a.skull_width*0.25 {
                if long { 0.007 } else { 0.003 }
            } else { if long { 0.004 } else { 0.0025 } }
        } else if long && tail {
            if guard { 0.070 } else { 0.018 }
        } else if long && ruff {
            if guard { 0.060 } else { 0.012 }
        } else if limb {
            if long { 0.004 } else { 0.0028 }
        } else if long && belly {
            if guard { 0.055 } else { 0.010 }
        } else if long {
            if guard { 0.030 } else { 0.006 }
        } else if plush {
            if guard { 0.015 } else { 0.007 }
        } else {
            if guard { 0.006 } else { 0.0025 }
        } * (0.7 + random(&mut seed) * 0.6);
        let normal = n.normalize();
        let flow = if face {
            Vec3::new(p.x.signum()*0.50, -0.24, -0.35)
        } else {
            Vec3::new(0.0, -0.35, -0.6)
        };
        let comb = (flow - normal * normal.dot(flow)).normalize_or_zero();
        let direction = (normal * 0.53 + comb * 0.85).normalize();
        samples.push((
            Root {
                position: p,
                normal,
                direction,
                bone: j[dominant] as usize,
                length,
                draped: long && !face && !paw,
                clump_offset: Vec3::ZERO,
            },
            color,
        ));
    }
    // Microclumps have independent pinned follicles. Only the distal curve
    // converges, and only along the local skin tangent plane, so clumping does
    // not move roots or press hair through the supporting surface.
    let mut clumps=std::collections::HashMap::new();
    if long_coat {
        for (root,_) in &mut samples {
            if !root.draped {continue;}
            let cell=(root.position/0.008).floor().as_ivec3();
            let key=(cell.x,cell.y,cell.z,root.bone);
            let anchor=*clumps.entry(key).or_insert(*root);
            let delta=anchor.rest_point(1.0)-root.rest_point(1.0);
            let lateral=delta-root.normal*delta.dot(root.normal);
            root.clump_offset=(lateral*0.65).clamp_length_max(root.length*0.20);
        }
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
        // Ribbon width must cross the strand, not a fixed world-up axis.
        // The old basis collapsed laterally combed cheek hairs edge-on.
        let across=root.normal.cross(root.direction);
        let tangent=if across.length_squared()>1e-12 {across.normalize()}
            else {root.normal.cross(if root.normal.y.abs()>0.9 {Vec3::X}else{Vec3::Y}).normalize()};
        let (skin_joints, skin_weights) = asset::skin(root.position, cat);
        // A rigid guide cannot reproduce a blended shoulder attachment. Keep
        // transition hairs on the same skin palette as their supporting body.
        let rigid_attachment = skin_weights.iter().copied().max().unwrap_or(0) > 62_258;
        let start = result.vertices.len() as u32;
        let segments=if root.draped {4} else {2};
        for segment in 0..=segments {
            let t = segment as f32 / segments as f32;
            let center = root.rest_point(t);
            let width = 0.00012 * (1.0 - t * 0.80);
            let joint = root.bone;
            for side in [-1.0, 1.0] {
                let mut vertex = super::super::actor_vertex(
                    (center + tangent * width * side).to_array(),
                    root.normal.to_array(),
                    {
                        let tabby=matches!(asset::breed(cat),voxy_model::animals::Breed::DomesticShorthair|voxy_model::animals::Breed::MaineCoon);
                        let band=if tabby {
                            let bright=((t-0.10)/0.25).clamp(0.,1.);
                            let tip=((t-0.65)/0.35).clamp(0.,1.);
                            0.72+0.38*bright-0.56*tip
                        } else {0.95+t*0.05};
                        let mut uv = asset::encoded_color(color * band, 0.86);
                        // Animal fibres carry the supporting skin normal.
                        // Their comb tangent is reconstructed in the shader;
                        // face and body use opposite longitudinal flow.
                        let a = asset::profile(cat);
                        let face = facial_region(a, root.position);
                        uv[1] -= if face { 5000.0 } else { 3000.0 };
                        uv[1] -= 0.0625 + t * 0.125 + if side < 0.0 { 0.0 } else { 0.5 };
                        uv
                    },
                    joint as u16,
                );
                if segment == 0 || !rigid_attachment {
                    vertex.joints = skin_joints;
                    vertex.weights = skin_weights;
                } else {
                    if let Some(guide) = guide {
                        let influence = ((root.length / result.roots[guide].length).min(1.0)
                            * 65535.0)
                            .round() as u16;
                        let tip = (influence as f32 * (t*2.-1.).max(0.)).round() as u16;
                        let mid = if t>=0.5 {influence-tip} else {(influence as f32*t*2.).round() as u16};
                        let remaining=65535_u32-mid as u32-tip as u32;
                        vertex.joints = [(BONES+guide*2) as u16,(BONES+guide*2+1) as u16,root.bone as u16,0];
                        vertex.weights = [mid,tip,remaining as u16,0];
                    }
                }
                result.vertices.push(vertex);
            }
        }
        for s in 0..segments {
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
#[derive(Debug)]
pub struct CoatDynamics {
    pub strands: Vec<Strand>,
    initialized: bool,
}
impl Clone for CoatDynamics {
    fn clone(&self) -> Self {
        Self {
            strands: self.strands.clone(),
            initialized: self.initialized,
        }
    }
    fn clone_from(&mut self, source: &Self) {
        self.strands.clone_from(&source.strands);
        self.initialized = source.initialized;
    }
}
impl CoatDynamics {
    pub fn new() -> Self {
        Self {
            strands: Vec::new(),
            initialized: false,
        }
    }
    pub fn step(&mut self, groom: &Groom, pose: &[Mat4; BONES], dt: f32) -> Result<(), String> {
        self.step_with_boxes(groom,pose,dt,&[])
    }
    pub fn step_with_boxes(&mut self, groom: &Groom, pose: &[Mat4; BONES], dt: f32, boxes: &[BoxCollider]) -> Result<(), String> {
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
                                        root.rest_point(i as f32 / 2.0),
                                    )
                                    .as_dvec3()
                                    .to_array()
                            })
                            .collect(),
                        StrandConfig {
                            stiffness: 650.0,
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
                .step_oriented_with_boxes(
                    f64::from(dt),
                    position.as_dvec3().to_array(),
                    [0.6, -9.81, 0.2],
                    &[collider],
                    boxes,
                    frame,
                )
                .map_err(str::to_string)?;
        }
        Ok(())
    }
    /// Append directly in actor space; retain the original multiply order.
    pub fn append_matrices(
        &self,
        groom: &Groom,
        pose: &[Mat4; BONES],
        actor_inverse: Mat4,
        out: &mut Vec<Mat4>,
    ) {
        out.reserve(pose.len() + groom.roots.len() * 2);
        out.extend(pose.iter().map(|matrix| actor_inverse * *matrix));
        for (index, root) in groom.roots.iter().enumerate() {
            let bone = pose[root.bone];
            for segment in 1..=2 {
                let rest = bone.transform_point3(
                    root.rest_point(segment as f32 / 2.),
                );
                let delta = self.strands.get(index).map_or(Vec3::ZERO, |s| {
                    Vec3::from_array(s.positions()[segment].map(|v| v as f32)) - rest
                });
                out.push(actor_inverse * (Mat4::from_translation(delta) * bone));
            }
        }
    }
    pub fn matrices(&self, groom: &Groom, pose: &[Mat4; BONES]) -> Vec<Mat4> {
        let mut matrices = pose.to_vec();
        for (index, root) in groom.roots.iter().enumerate() {
            let bone = pose[root.bone];
            for segment in 1..=2 {
                let rest = bone.transform_point3(
                    root.rest_point(segment as f32 / 2.0),
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

pub fn environment_boxes(world: &World, bodies: &[AnchoredAabb], groom: &Groom, pose: &[Mat4; BONES]) -> Result<Vec<BoxCollider>,String> {
    use voxy_world::{VoxelView,Sample,CollisionShape};
    let mut min=Vec3::splat(f32::INFINITY);
    let mut max=Vec3::splat(f32::NEG_INFINITY);
    for root in &groom.roots {
        for t in [0.0,0.5,1.0] {
            let p=pose[root.bone].transform_point3(root.rest_point(t));
            min=min.min(p); max=max.max(p);
        }
    }
    let lo=(min-Vec3::splat(0.2)).floor().as_ivec3();
    let hi=(max+Vec3::splat(0.2)).floor().as_ivec3();
    let size=hi.as_i64vec3()-lo.as_i64vec3()+glam::I64Vec3::ONE;
    if size.min_element()<=0 || size.max_element()>4096 || size.x*size.y*size.z>4096 {return Err("fur environment query budget exceeded".into());}
    let mut boxes=Vec::new();
    for x in lo.x..=hi.x {for y in lo.y..=hi.y {for z in lo.z..=hi.z {
        let solid=match world.sample(VoxelPos{x:i64::from(x),y:i64::from(y),z:i64::from(z)}) {
            Sample::Loaded(block)=>world.registry().get(block).ok_or("unknown fur-contact block")?.collision!=CollisionShape::Empty,
            Sample::Unloaded{..}|Sample::Unavailable{..}=>true,
        };
        if solid { boxes.push(BoxCollider{min:[f64::from(x),f64::from(y),f64::from(z)],max:[f64::from(x)+1.0,f64::from(y)+1.0,f64::from(z)+1.0]}); }
    }}}
    for body in bodies {
        let origin=[body.anchor.x as f64,body.anchor.y as f64,body.anchor.z as f64];
        let b=BoxCollider{min:std::array::from_fn(|i|origin[i]+body.min[i]),max:std::array::from_fn(|i|origin[i]+body.max[i])};
        if (0..3).all(|i| b.max[i]>=f64::from(min[i])-0.2 && b.min[i]<=f64::from(max[i])+0.2) {boxes.push(b);}
    }
    Ok(boxes)
}

fn rest_frame(matrix: Mat4) -> [[f64; 3]; 3] {
    let x = matrix.x_axis.truncate().as_dvec3().normalize();
    let z = x.cross(matrix.y_axis.truncate().as_dvec3()).normalize();
    let y = z.cross(x);
    [x.to_array(), y.to_array(), z.to_array()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facial_ribbons_have_width_across_the_strand() {
        for cat in [false,true] {
            let groom=build(cat).unwrap();
            let mut count=0;
            let mut worst=0.0_f32;
            for hair in groom.vertices.chunks_exact(6) {
                if hair[0].uv[1] > -15000. {continue;}
                let position=|i:usize| Vec3::from(hair[i].position);
                let root=(position(0)+position(1))*0.5;
                let tip=(position(4)+position(5))*0.5;
                let across=(position(1)-position(0)).normalize();
                let along=(tip-root).normalize();
                worst=worst.max(across.dot(along).abs());
                count+=1;
            }
            assert!(count>1000,"missing facial fibres: {count}");
            assert!(worst<0.01,"ribbon width runs along the hair: {worst}");
        }
    }
    #[test]
    fn voxel_floor_and_neighbour_bodies_are_included_in_fur_contacts() {
        let mut scene=voxy_runtime::build_procedural_scene(42,1).unwrap();
        super::super::prepare_check_scene(&mut scene).unwrap();
        let root=Root{position:Vec3::new(18.0,13.02,18.0),normal:Vec3::Y,
            direction:Vec3::new(1.0,-0.6,0.0).normalize(),bone:0,length:0.12,
            draped:false,clump_offset:Vec3::ZERO};
        let groom=Groom{roots:vec![root],vertices:Vec::new(),indices:Vec::new()};
        let pose=[Mat4::IDENTITY;BONES];
        let boxes=environment_boxes(&scene.world,&[],&groom,&pose).unwrap();
        assert!(boxes.iter().any(|b|b.min==[18.,12.,18.]&&b.max==[19.,13.,19.]));
        let neighbour=AnchoredAabb{anchor:VoxelPos{x:18,y:13,z:18},min:[0.04,0.0,-0.04],max:[0.08,0.1,0.04]};
        let with_body=environment_boxes(&scene.world,&[neighbour],&groom,&pose).unwrap();
        assert_eq!(with_body.len(),boxes.len()+1);
        let mut coat=CoatDynamics::new();
        for _ in 0..30 {
            coat.step_with_boxes(&groom,&pose,1.0/60.0,&boxes).unwrap();
            assert_eq!(coat.strands[0].positions()[0],[18.,f64::from(root.position.y),18.]);
            assert!(coat.strands[0].positions().iter().skip(1).all(|p|p[1]>=13.0003-1e-9));
        }
    }

    #[test]
    fn facial_groom_does_not_shorten_the_chest_ruff() {
        for breed in voxy_model::animals::Breed::ALL {
            let a=breed.anatomy();
            let skull=a.head_length-a.muzzle_length;
            assert!(facial_region(a,a.head_origin()+Vec3::new(0.0,0.0,skull*0.5)));
            assert!(!facial_region(a,Vec3::new(0.0,a.shoulder_height*0.85,a.trunk_length*0.36)));
        }
    }

    #[test]
    fn hair_roots_follow_blended_skin_when_legs_move() {
        let pose: [Mat4; BONES] = std::array::from_fn(|i| {
            Mat4::from_rotation_translation(
                Quat::from_rotation_x(i as f32 * 0.07),
                Vec3::new(0.0, i as f32 * 0.003, i as f32 * 0.002),
            )
        });
        for cat in [false, true] {
            let groom = groom(cat).unwrap();
            let mut blended = 0;
            for hair in groom.vertices.chunks_exact(6) {
                let root = (Vec3::from(hair[0].position) + Vec3::from(hair[1].position)) * 0.5;
                let (joints, weights) = asset::skin(root, cat);
                let expected: Vec3 = (0..4).map(|k| {
                    pose[joints[k] as usize].transform_point3(root) * (weights[k] as f32 / 65535.0)
                }).sum();
                let actual: Vec3 = hair[..2].iter().map(|v| {
                    (0..4).map(|k| {
                        assert!((v.joints[k] as usize) < BONES);
                        pose[v.joints[k] as usize].transform_point3(Vec3::from(v.position))
                            * (v.weights[k] as f32 / 65535.0)
                    }).sum::<Vec3>() * 0.5
                }).sum();
                assert!(actual.distance(expected) < 0.00002, "detached hair root");
                blended += usize::from(weights.iter().filter(|&&w| w > 1000).count() > 1);
            }
            assert!(blended > 1000, "exercise muscle attachment regions");
        }
    }
}
