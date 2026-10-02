//! Model-space humanoid rig with anatomical weights and volume-preserving skinning.
use glam::{Mat4, Quat, Vec3};
use voxy_animation::{
    AnimationClip, Joint, JointTrack, Playback, Pose, QuatKey, Skeleton, Transform,
};
use voxy_render::SceneVertex;

/// Object in the canonical left hand's rest frame. Meshes must be closed and outward wound.
#[derive(Debug)]
pub(crate) enum GraspObject {
    Sphere {
        center: Vec3,
        radius: f32,
    },
    Cylinder {
        center: Vec3,
        radius: f32,
        half_length: f32,
    },
    Mesh {
        surface: physics::hair::TriangleMesh,
        triangles: Vec<[Vec3; 3]>,
        bounds: [Vec3; 2],
    },
}
impl GraspObject {
    /// Validates surface topology after welding OBJ position seams.
    /// Shells must be closed and manifold, with normals pointing out of material.
    /// Nested void walls have the opposite orientation from outer shells.
    pub(crate) fn from_mesh(
        vertices: &[SceneVertex],
        indices: &[u32],
    ) -> Result<Self, &'static str> {
        if vertices.is_empty()
            || indices.is_empty()
            || indices.len() % 3 != 0
            || indices.iter().any(|&i| i as usize >= vertices.len())
        {
            return Err("invalid grasp mesh indices");
        }
        let mut lookup = std::collections::BTreeMap::new();
        let mut points = Vec::new();
        let mut welded = Vec::with_capacity(vertices.len());
        for vertex in vertices {
            let point = Vec3::from_array(vertex.position);
            if !point.is_finite() {
                return Err("nonfinite grasp mesh position");
            }
            let key = vertex
                .position
                .map(|x| if x == 0. { 0 } else { x.to_bits() });
            let id = *lookup.entry(key).or_insert_with(|| {
                let id = points.len();
                points.push(point);
                id
            });
            welded.push(id);
        }
        let mut faces = Vec::new();
        for triangle in indices.chunks_exact(3) {
            let face = [
                welded[triangle[0] as usize],
                welded[triangle[1] as usize],
                welded[triangle[2] as usize],
            ];
            let [a, b, c] = face.map(|i| points[i].as_dvec3());
            if (b - a).cross(c - a).length_squared() == 0. {
                return Err("degenerate grasp mesh triangle");
            }
            faces.push(face);
        }
        let mut edges = std::collections::BTreeMap::new();
        let mut adjacency = vec![Vec::new(); faces.len()];
        for (id, face) in faces.iter().enumerate() {
            for edge in 0..3 {
                let a = face[edge];
                let b = face[(edge + 1) % 3];
                let key = (a.min(b), a.max(b));
                match edges.entry(key) {
                    std::collections::btree_map::Entry::Vacant(e) => {
                        e.insert((id, a < b, false));
                    }
                    std::collections::btree_map::Entry::Occupied(mut e) => {
                        let (other, direction, sealed) = *e.get();
                        if sealed {
                            return Err("nonmanifold grasp mesh edge");
                        }
                        if direction == (a < b) {
                            return Err("inconsistent grasp mesh winding");
                        }
                        adjacency[id].push(other);
                        adjacency[other].push(id);
                        e.get_mut().2 = true;
                    }
                }
            }
        }
        if edges.values().any(|entry| !entry.2) {
            return Err("open grasp mesh surface");
        }
        let mut incident = vec![Vec::new(); points.len()];
        for (id, face) in faces.iter().enumerate() {
            for &vertex in face {
                incident[vertex].push(id);
            }
        }
        for (vertex, fan) in incident.iter().enumerate() {
            if fan.is_empty() {
                continue;
            }
            let mut seen = std::collections::BTreeSet::new();
            let mut pending = vec![fan[0]];
            seen.insert(fan[0]);
            while let Some(id) = pending.pop() {
                for &neighbor in &adjacency[id] {
                    if faces[neighbor].contains(&vertex) && seen.insert(neighbor) {
                        pending.push(neighbor);
                    }
                }
            }
            if seen.len() != fan.len() {
                return Err("nonmanifold grasp mesh vertex");
            }
        }
        let triangles: Vec<_> = faces.iter().map(|f| f.map(|i| points[i])).collect();
        let mut components = Vec::new();
        let mut volumes = Vec::new();
        let mut visited = vec![false; faces.len()];
        for first in 0..faces.len() {
            if visited[first] {
                continue;
            }
            let origin = points[faces[first][0]].as_dvec3();
            let mut pending = vec![first];
            visited[first] = true;
            let mut volume = 0_f64;
            let mut component = Vec::new();
            while let Some(id) = pending.pop() {
                component.push(id);
                let [a, b, c] = faces[id].map(|i| points[i].as_dvec3() - origin);
                volume += a.dot(b.cross(c));
                for &neighbor in &adjacency[id] {
                    if !visited[neighbor] {
                        visited[neighbor] = true;
                        pending.push(neighbor);
                    }
                }
            }
            if volume == 0. || !volume.is_finite() {
                return Err("zero-volume grasp mesh shell");
            }
            volumes.push(volume);
            components.push(component);
        }
        for (id, component) in components.iter().enumerate() {
            let point = triangles[component[0]][0];
            let depth = components
                .iter()
                .enumerate()
                .filter(|(other, faces)| {
                    *other != id
                        && faces
                            .iter()
                            .map(|&face| solid_angle(triangles[face], point))
                            .sum::<f64>()
                            .abs()
                            > std::f64::consts::TAU
                })
                .count();
            if (volumes[id] > 0.) != (depth % 2 == 0) {
                return Err("grasp mesh shell faces into material");
            }
        }
        let bounds = [
            points
                .iter()
                .copied()
                .fold(Vec3::splat(f32::INFINITY), Vec3::min),
            points
                .iter()
                .copied()
                .fold(Vec3::splat(f32::NEG_INFINITY), Vec3::max),
        ];
        let positions: Vec<_> = points.iter().map(|p| p.to_array().map(f64::from)).collect();
        Ok(Self::Mesh {
            surface: physics::hair::TriangleMesh::new(&positions, &faces)?,
            triangles,
            bounds,
        })
    }
    fn closest_point(&self, point: Vec3) -> Vec3 {
        match self {
            Self::Sphere { center, radius } => {
                let delta = point - *center;
                *center
                    + if delta.length_squared() > 0. {
                        delta.normalize() * *radius
                    } else {
                        Vec3::X * *radius
                    }
            }
            Self::Cylinder {
                center,
                radius,
                half_length,
            } => {
                let delta = point - *center;
                let xy = delta.truncate();
                let length = xy.length();
                let direction = if length > 0. {
                    xy / length
                } else {
                    glam::Vec2::X
                };
                let mut radial = direction * length.min(*radius);
                let mut z = delta.z.clamp(-*half_length, *half_length);
                if length < *radius && delta.z.abs() < *half_length {
                    if radius - length < half_length - delta.z.abs() {
                        radial = direction * *radius;
                    } else {
                        z = if delta.z < 0. {
                            -*half_length
                        } else {
                            *half_length
                        };
                    }
                }
                *center + Vec3::new(radial.x, radial.y, z)
            }
            Self::Mesh { surface, .. } => Vec3::from_array(
                surface
                    .closest_surface(point.to_array().map(f64::from))
                    .expect("finite finger target")
                    .0
                    .map(|v| v as f32),
            ),
        }
    }
    fn distance(&self, point: Vec3) -> f32 {
        match self {
            Self::Sphere { center, radius } => point.distance(*center) - radius,
            Self::Cylinder {
                center,
                radius,
                half_length,
            } => {
                let p = point - *center;
                let radial = p.truncate().length() - radius;
                let axial = p.z.abs() - half_length;
                glam::Vec2::new(radial.max(0.), axial.max(0.)).length() + radial.max(axial).min(0.)
            }
            Self::Mesh {
                surface: mesh,
                triangles,
                bounds,
            } => {
                let (surface, _) = mesh
                    .closest_surface(point.to_array().map(f64::from))
                    .expect("finite hand position");
                let delta = point - Vec3::from_array(surface.map(|x| x as f32));
                if point.cmplt(bounds[0]).any() || point.cmpgt(bounds[1]).any() {
                    return delta.length();
                }
                // Nearest normals cannot classify inside at mesh edges.
                let winding: f64 = triangles.iter().map(|&face| solid_angle(face, point)).sum();
                delta.length()
                    * if winding.abs() > std::f64::consts::TAU {
                        -1.
                    } else {
                        1.
                    }
            }
        }
    }
}

fn solid_angle(face: [Vec3; 3], point: Vec3) -> f64 {
    let [a, b, c] = face.map(|v| (v - point).as_dvec3());
    let numerator = a.dot(b.cross(c));
    let denominator = a.length() * b.length() * c.length()
        + a.dot(b) * c.length()
        + b.dot(c) * a.length()
        + c.dot(a) * b.length();
    2. * numerator.atan2(denominator)
}

#[derive(Clone, Debug)]
pub(crate) struct PreparedGrasp {
    object: Option<std::sync::Arc<GraspObject>>,
    samples: Option<std::sync::Arc<Vec<[[Quat; 3]; 5]>>>,
}
#[derive(Debug)]
pub(crate) struct FemaleRig {
    pub skeleton: Skeleton,
    clip: AnimationClip,
    grasp_clip: AnimationClip,
    grasp_mix: f32,
    grasp_cycle: bool,
    body_motion: bool,
    grasp_object: Option<std::sync::Arc<GraspObject>>,
    grasp_samples: Option<std::sync::Arc<Vec<[[Quat; 3]; 5]>>>,
    hand_edges: Vec<(usize, usize, f32)>,
    hand_faces: Vec<([usize; 3], f32)>,
    thumb_web_hinges: Vec<([usize; 4], f32)>,
    hand_mobility: Vec<f32>,
    weights: Vec<[(usize, f32); 4]>,
    finger_weights: Vec<Option<Box<[(usize, f32)]>>>,
    finger_contact_skin: [Vec<(Vec3, Box<[(usize, f32)]>)>; 5],
}
impl FemaleRig {
    pub fn new(vertices: &[SceneVertex]) -> Result<Self, voxy_animation::AnimationError> {
        let mut bones = vec![
            (
                "pelvis",
                None,
                Vec3::new(0., -0.10, 0.),
                Vec3::new(0., 0.08, 0.),
            ),
            (
                "spine",
                Some(0),
                Vec3::new(0., 0.08, 0.),
                Vec3::new(0., 0.30, 0.),
            ),
            (
                "chest",
                Some(1),
                Vec3::new(0., 0.30, 0.),
                Vec3::new(0., 0.48, 0.),
            ),
            (
                "head",
                Some(2),
                Vec3::new(0., 0.48, 0.),
                Vec3::new(0., 0.78, 0.),
            ),
        ];
        let mut bones: Vec<_> = bones
            .drain(..)
            .map(|(n, p, a, b)| (n.to_owned(), p, a, b))
            .collect();
        for (side, s) in [("left", 1.), ("right", -1.)] {
            let start = bones.len();
            for (name, parent, a, b) in [
                (
                    format!("{side}_upper_arm"),
                    2,
                    Vec3::new(s * 0.17, 0.43, 0.),
                    Vec3::new(s * 0.29, 0.20, 0.),
                ),
                (
                    format!("{side}_forearm"),
                    start,
                    Vec3::new(s * 0.29, 0.20, 0.),
                    Vec3::new(s * 0.38, -0.02, 0.),
                ),
                (
                    format!("{side}_hand"),
                    start + 1,
                    Vec3::new(s * 0.38, -0.02, 0.),
                    Vec3::new(s * 0.40, -0.14, 0.),
                ),
                (
                    format!("{side}_thigh"),
                    0,
                    Vec3::new(s * 0.09, -0.10, 0.),
                    Vec3::new(s * 0.10, -0.44, 0.),
                ),
                (
                    format!("{side}_shin"),
                    start + 3,
                    Vec3::new(s * 0.10, -0.44, 0.),
                    Vec3::new(s * 0.10, -0.74, 0.),
                ),
                (
                    format!("{side}_foot"),
                    start + 4,
                    Vec3::new(s * 0.10, -0.74, 0.),
                    Vec3::new(s * 0.10, -0.79, 0.12),
                ),
            ] {
                bones.push((name, Some(parent), a, b));
            }
        }
        for (side_index,(side,sign)) in [("left",1.),("right",-1.)].into_iter().enumerate() {
            for finger in 1..5 {
                let z=[0.070,0.055,0.040,0.025][finger-1];
                bones.push((format!("{side}_metacarpal_{finger}"),Some(if side_index==0{6}else{12}),Vec3::new(sign*0.35,0.005,z),Vec3::new(sign*FINGER_CHAINS[finger][0][0],FINGER_CHAINS[finger][0][1],FINGER_CHAINS[finger][0][2])));
            }
        }
        for (side_index, (side, sign)) in [("left", 1.), ("right", -1.)].into_iter().enumerate() {
            let hand = if side_index == 0 { 6 } else { 12 };
            for (finger_index, chain) in FINGER_CHAINS.iter().enumerate() {
                let start = bones.len();
                for segment in 0..3 {
                    let mirror = |point: [f32; 3]| Vec3::new(sign * point[0], point[1], point[2]);
                    bones.push((
                        format!("{side}_finger_{finger_index}_{segment}"),
                        Some(if segment == 0 {
                            if finger_index==0 {hand} else {16+side_index*4+finger_index-1}
                        } else {
                            start + segment - 1
                        }),
                        mirror(chain[segment]),
                        mirror(chain[segment + 1]),
                    ));
                }
            }
        }
        let skeleton = Skeleton::new(
            bones
                .iter()
                .map(|(name, parent, a, _)| Joint {
                    name: name.as_str().into(),
                    parent: parent.map(|i| i as u16),
                    bind_local: Transform {
                        translation: *a - parent.map_or(Vec3::ZERO, |i| bones[i].2),
                        ..Transform::IDENTITY
                    },
                    inverse_bind: Mat4::from_translation(-*a),
                })
                .collect(),
        )?;
        let weights = vertices
            .iter()
            .map(|v| anatomical_weights(Vec3::from_array(v.position)))
            .collect();
        let finger_weights: Vec<Option<Box<[(usize, f32)]>>> = vertices
            .iter()
            .map(|v| finger_weights(Vec3::from_array(v.position)).map(Vec::into_boxed_slice))
            .collect();
        let finger_contact_skin = std::array::from_fn(|finger| {
            let start = 24 + finger * 3;
            vertices
                .iter()
                .zip(&finger_weights)
                .filter_map(|(vertex, weights)| {
                    let point = Vec3::from_array(vertex.position);
                    if point.x <= 0. {
                        return None;
                    }
                    let weights = weights.as_ref()?;
                    let influence: f32 = weights
                        .iter()
                        .filter(|(bone, _)| (start..start + 3).contains(bone))
                        .map(|(_, weight)| weight)
                        .sum();
                    (influence >= if finger==0{0.01}else{0.8}).then(|| (point, weights.clone()))
                })
                .collect()
        });
        let tracks: Vec<JointTrack> = bones
            .iter()
            .map(|(name, parent, origin, tip)| {
                JointTrack {
                    rotations: (0..=120)
                        .map(|i| {
                            let phase = i as f32 * std::f32::consts::TAU / 120.;
                            let time = i as f32 * 0.05;
                            let ease = gesture_envelope(time);
                            let sign = if name.starts_with("right") { -1. } else { 1. };
                            // The hinge lies perpendicular to the upper arm and forward axis.
                            // Using world X twists this A-pose arm while bending it.
                            let rotation = if name.contains("_finger_") {
                                let segment =
                                    name.rsplit('_').next().unwrap().parse::<usize>().unwrap();
                                let axis =
                                    (*tip - *origin).cross(Vec3::new(-sign, 0., 0.)).normalize();
                                let angle = [0.16, 0.22, 0.16][segment]
                                    * if name.contains("_finger_0_") {
                                        0.22
                                    } else {
                                        1.
                                    };
                                Quat::from_axis_angle(
                                    axis,
                                    angle * gesture_envelope(time - 0.25 - segment as f32 * 0.04),
                                )
                            } else if name.ends_with("forearm") {
                                let upper = *origin - bones[parent.expect("forearm parent")].2;
                                let axis = upper.cross(Vec3::Z).normalize();
                                let amplitude = if sign > 0. { 0.75 } else { 0.24 };
                                Quat::from_axis_angle(
                                    axis,
                                    amplitude * gesture_envelope(time - 0.12),
                                )
                            } else if name.ends_with("upper_arm") {
                                // Share the reach between shoulder and elbow instead of folding
                                // nearly all motion into a low, fixed upper arm.
                                let amplitude = if sign > 0. { 0.22 } else { 0.08 };
                                let axis = (*tip - *origin).cross(Vec3::Z).normalize();
                                Quat::from_axis_angle(axis, amplitude * ease)
                            } else if name.ends_with("hand") {
                                let axis = (*tip - *origin).normalize();
                                Quat::from_axis_angle(
                                    axis,
                                    sign * 0.06 * gesture_envelope(time - 0.22),
                                )
                            } else if name == "head" {
                                Quat::from_rotation_y(0.05 * phase.sin())
                            } else if name == "spine" {
                                Quat::from_rotation_y(0.015 * gesture_envelope(time + 0.15))
                            } else if name == "chest" {
                                Quat::from_rotation_y(0.025 * gesture_envelope(time + 0.10))
                            } else {
                                Quat::IDENTITY
                            };
                            QuatKey {
                                time: i as f32 * 0.05,
                                value: rotation,
                            }
                        })
                        .collect(),
                    ..JointTrack::default()
                }
            })
            .collect();
        let mut grasp_tracks = tracks.clone();
        for side in 0..2 {
            for finger in 0..5 {
                for segment in 0..3 {
                    grasp_tracks[24 + side * 15 + finger * 3 + segment].rotations = vec![QuatKey {
                        time: 0.,
                        value: grasp_rotation(finger, segment, if side == 0 { 1. } else { -1. }),
                    }];
                }
            }
        }
        let grasp_clip = AnimationClip::new(
            "cylindrical hand grasp",
            6.,
            Playback::Loop,
            grasp_tracks,
            &skeleton,
        )?;
        let clip =
            AnimationClip::new("relaxed arm gesture", 6., Playback::Loop, tracks, &skeleton)?;
        Ok(Self {
            skeleton,
            clip,
            grasp_clip,
            grasp_mix: 0.,
            grasp_cycle: false,
            body_motion: true,
            grasp_object: None,
            grasp_samples: None,
            hand_edges: Vec::new(),
            hand_faces: Vec::new(),
            thumb_web_hinges: Vec::new(),
            hand_mobility: Vec::new(),
            weights,
            finger_weights,
            finger_contact_skin,
        })
    }
    /// Continuous hand closure: zero is the existing relaxed pose, one is a power grip.
    pub(crate) fn set_grasp(&mut self, amount: f32) -> Result<(), &'static str> {
        if !amount.is_finite() || !(0. ..=1.).contains(&amount) {
            return Err("grasp must be in 0..1");
        }
        self.grasp_mix = amount;
        self.grasp_cycle = false;
        Ok(())
    }
    pub(crate) fn set_grasp_object(&mut self, object: Option<std::sync::Arc<GraspObject>>) {
        self.grasp_samples = object.as_ref().map(|object| {
            let adduction: [Quat; 5] = std::array::from_fn(|finger| {
                if finger == 0 {
                    // Use the smallest additional opposition that reaches this object.
                    let mut best = (f32::INFINITY, Quat::IDENTITY);
                    for spread_step in 0..=100 {
                        let spread = ((spread_step + 1) / 2) as f32
                            * 0.005
                            * if spread_step % 2 == 0 { -1. } else { 1. };
                        for step in 0..=24 {
                            let opposition = step as f32 * 0.005;
                            let correction =
                                Quat::from_rotation_y(spread) * Quat::from_rotation_x(opposition);
                            let desired = std::array::from_fn(|joint| {
                                let rotation = grasp_rotation(0, joint, 1.);
                                if joint == 0 {
                                    correction * rotation
                                } else {
                                    rotation
                                }
                            });
                            let fitted =
                                fit_finger(FINGER_CHAINS[0], [Quat::IDENTITY; 3], desired, object);
                            let reference_clearance =
                                finger_clearance(FINGER_CHAINS[0], fitted, object);
                            let clearance = if reference_clearance <= 0.00001 {
                                let path = prepare_finger_grasp(FINGER_CHAINS[0], desired, object);
                                finger_clearance(FINGER_CHAINS[0], path[120], object)
                            } else {
                                reference_clearance
                            };
                            if clearance < best.0 {
                                best = (clearance, correction);
                            }
                            if clearance <= 0.00001 {
                                return correction;
                            }
                        }
                    }
                    return best.1;
                }
                let chain = FINGER_CHAINS[finger];
                let tip = Vec3::from_array(chain[3]);
                let target = object.closest_point(tip);
                let length: f32 = chain
                    .windows(2)
                    .map(|p| Vec3::from_array(p[0]).distance(Vec3::from_array(p[1])))
                    .sum();
                let limit = if finger == 4 { 0.50 } else { 0.35 };
                let initial = (-(target.z - tip.z) / length).atan().clamp(-limit, limit);
                if finger != 4 {
                    return Quat::from_rotation_x(initial);
                }
                let mut best = (f32::INFINITY, initial);
                for step in 0..=40 {
                    let offset =
                        ((step + 1) / 2) as f32 * 0.01 * if step % 2 == 0 { -1. } else { 1. };
                    let angle = (initial + offset).clamp(-0.60, 0.60);
                    let desired = std::array::from_fn(|joint| {
                        let (axis, _) = grasp_rotation(finger, joint, 1.).to_axis_angle();
                        let bend = Quat::from_axis_angle(
                            axis,
                            if matches!(object.as_ref(), GraspObject::Mesh { .. }) {
                                [0.95, 1.40, 0.95][joint]
                            } else {
                                [0.75, 1.10, 0.65][joint]
                            },
                        );
                        if joint == 0 {
                            Quat::from_rotation_x(angle) * bend
                        } else {
                            bend
                        }
                    });
                    let fitted = fit_finger(chain, [Quat::IDENTITY; 3], desired, object);
                    let clearance = finger_clearance_from(chain, fitted, object, 2);
                    if clearance < best.0 {
                        best = (clearance, angle);
                    }
                    if clearance <= 0.00001 {
                        break;
                    }
                }
                Quat::from_rotation_x(best.1)
            });
            let fingers: [Vec<[Quat; 3]>; 5] = std::array::from_fn(|finger| {
                let desired = std::array::from_fn(|joint| {
                    let rotation = grasp_rotation(finger, joint, 1.);
                    let rotation = if finger == 0 {
                        rotation
                    } else {
                        let (axis, _) = rotation.to_axis_angle();
                        Quat::from_axis_angle(
                            axis,
                            if matches!(object.as_ref(), GraspObject::Mesh { .. }) {
                                [0.95, 1.40, 0.95][joint]
                            } else {
                                [0.75, 1.10, 0.65][joint]
                            },
                        )
                    };
                    if joint == 0 {
                        adduction[finger] * rotation
                    } else {
                        rotation
                    }
                });
                let desired =
                    if finger != 0 && matches!(object.as_ref(), GraspObject::Cylinder { .. }) {
                        cylinder_curled_contact_pose(
                            FINGER_CHAINS[finger],
                            finger,
                            object,
                            &self.finger_contact_skin[finger],
                            adduction[finger],
                        )
                        .unwrap_or(desired)
                    } else {
                        desired
                    };
                let path = prepare_finger_grasp(FINGER_CHAINS[finger], desired, object);
                if finger == 0
                    && matches!(
                        object.as_ref(),
                        GraspObject::Sphere { .. } | GraspObject::Mesh { .. }
                    )
                {
                    curled_thumb_contact_path(&path, object, &self.finger_contact_skin[0])
                        .unwrap_or(path)
                } else if finger == 0 && matches!(object.as_ref(), GraspObject::Cylinder { .. }) {
                    cylinder_thumb_contact_path(&path, object, &self.finger_contact_skin[0])
                        .unwrap_or(path)
                } else {
                    path
                }
            });
            let samples = (0..=120)
                .map(|frame| std::array::from_fn(|finger| fingers[finger][frame]))
                .collect();
            std::sync::Arc::new(samples)
        });
        self.grasp_object = object;
    }
    pub(crate) fn prepared_grasp(&self) -> PreparedGrasp {
        PreparedGrasp {
            object: self.grasp_object.clone(),
            samples: self.grasp_samples.clone(),
        }
    }
    pub(crate) fn apply_prepared_grasp(&mut self, prepared: &PreparedGrasp) {
        self.grasp_object = prepared.object.clone();
        self.grasp_samples = prepared.samples.clone();
    }
    pub(crate) fn copy_grasp_target_from(&mut self, source: &Self) {
        self.grasp_object = source.grasp_object.clone();
        self.grasp_samples = source.grasp_samples.clone();
    }
    pub(crate) fn set_grasp_cycle(&mut self, enabled: bool) {
        self.set_grasp(0.).expect("valid neutral grasp");
        self.grasp_cycle = enabled;
    }
    pub(crate) fn set_body_motion(&mut self, enabled: bool) {
        self.body_motion = enabled;
    }
    /// Bind surface edges used by the grasp skin corrective. The wrist stays anchored.
    pub(crate) fn bind_hand_surface(
        &mut self,
        vertices: &[SceneVertex],
        indices: &[u32],
    ) -> Result<(), &'static str> {
        if vertices.len() != self.weights.len()
            || indices.len() % 3 != 0
            || indices.iter().any(|&i| i as usize >= vertices.len())
        {
            return Err("invalid hand surface binding");
        }
        self.hand_mobility = vertices
            .iter()
            .map(|v| {
                let p = Vec3::from_array(v.position);
                smooth(0.32, 0.35, p.x.abs())
                    * smooth(-0.020, 0.025, -p.y)
                    * (1. - smooth(0.14, 0.16, -p.y))
            })
            .collect();
        let mut edges = std::collections::BTreeSet::new();
        for face in indices.chunks_exact(3) {
            for i in 0..3 {
                let a = face[i] as usize;
                let b = face[(i + 1) % 3] as usize;
                if self.hand_mobility[a] > 0. || self.hand_mobility[b] > 0. {
                    edges.insert((a.min(b), a.max(b)));
                }
            }
        }
        self.hand_edges = edges
            .into_iter()
            .map(|(a, b)| {
                (
                    a,
                    b,
                    Vec3::from_array(vertices[a].position)
                        .distance(Vec3::from_array(vertices[b].position)),
                )
            })
            .collect();
        self.hand_faces = indices
            .chunks_exact(3)
            .filter_map(|face| {
                let ids = [face[0] as usize, face[1] as usize, face[2] as usize];
                if ids.iter().all(|&i| self.hand_mobility[i] == 0.) {
                    return None;
                }
                let p = ids.map(|i| Vec3::from_array(vertices[i].position));
                let area = (p[1] - p[0]).cross(p[2] - p[0]).length();
                (area > 1e-10).then_some((ids, area))
            })
            .collect();
        let mut adjacent = std::collections::BTreeMap::new();
        self.thumb_web_hinges.clear();
        for face in indices.chunks_exact(3) {
            for j in 0..3 {
                let a = face[j] as usize;
                let b = face[(j + 1) % 3] as usize;
                let c = face[(j + 2) % 3] as usize;
                let edge = (a.min(b), a.max(b));
                if let Some(d) = adjacent.insert(edge, c) {
                    let ids = [edge.0, edge.1, c, d];
                    let p = ids.map(|i| Vec3::from_array(vertices[i].position));
                    let center = p.iter().copied().sum::<Vec3>() * 0.25;
                    if (0.33..0.37).contains(&center.x.abs())
                        && (-0.025..0.025).contains(&center.y)
                        && (0.070..0.115).contains(&center.z)
                    {
                        let normal_a = (p[1] - p[0]).cross(p[2] - p[0]).normalize();
                        let normal_b = (p[0] - p[1]).cross(p[3] - p[1]).normalize();
                        let angle = normal_a.dot(normal_b).clamp(-1., 1.).acos();
                        if angle.is_finite() {
                            self.thumb_web_hinges.push((ids, angle));
                        }
                    }
                }
            }
        }
        Ok(())
    }
    fn correct_grasp_skin(&self, vertices: &mut [SceneVertex], face_normals: &[Option<Vec3>]) {
        let inward_cylinder = self.inward_cylinder();
        let bend_enabled = !self.hand_faces.is_empty()
            && !self.hand_edges.is_empty()
            && (inward_cylinder
                || !matches!(
                    self.grasp_object.as_deref(),
                    Some(GraspObject::Cylinder { .. })
                ));
        for _ in 0..if inward_cylinder {
            192
        } else if bend_enabled {
            96
        } else {
            24
        } {
            if bend_enabled {
                for &(ids, rest_angle) in &self.thumb_web_hinges {
                    let p = ids.map(|i| Vec3::from_array(vertices[i].position));
                    let angle = |points: [Vec3; 4]| {
                        let a = (points[1] - points[0])
                            .cross(points[2] - points[0])
                            .normalize();
                        let b = (points[0] - points[1])
                            .cross(points[3] - points[1])
                            .normalize();
                        a.cross(b).length().atan2(a.dot(b))
                    };
                    let excess = angle(p) - rest_angle - 0.25;
                    if excess <= 0. {
                        continue;
                    }
                    let gradients: [Vec3; 4] = std::array::from_fn(|j| {
                        let components: [f32; 3] = std::array::from_fn(|axis| {
                            let mut plus = p;
                            let mut minus = p;
                            plus[j][axis] += 0.00001;
                            minus[j][axis] -= 0.00001;
                            (angle(plus) - angle(minus)) / 0.00002
                        });
                        Vec3::from_array(components)
                    });
                    let denominator: f32 = (0..4)
                        .map(|j| self.hand_mobility[ids[j]] * gradients[j].length_squared())
                        .sum();
                    if denominator <= 1e-12 {
                        continue;
                    }
                    for j in 0..4 {
                        let delta = -gradients[j]
                            * (0.30 * excess * self.hand_mobility[ids[j]] / denominator);
                        let delta = delta.clamp_length_max(0.00005);
                        vertices[ids[j]].position = (p[j] + delta).to_array();
                    }
                }
            }
            for &(a, b, rest) in &self.hand_edges {
                let pa = Vec3::from_array(vertices[a].position);
                let pb = Vec3::from_array(vertices[b].position);
                let delta = pb - pa;
                let length = delta.length();
                if length < 1e-8 {
                    continue;
                }
                let target = length.clamp(rest * 0.60, rest * 1.40);
                let mobility = self.hand_mobility[a] + self.hand_mobility[b];
                if target == length || mobility == 0. {
                    continue;
                }
                let correction = delta * ((length - target) / (length * mobility));
                vertices[a].position = (pa + correction * self.hand_mobility[a]).to_array();
                vertices[b].position = (pb - correction * self.hand_mobility[b]).to_array();
            }
            // Edge bounds alone allow a joint triangle to flatten into a crease.
            // Restore its area locally, sharing displacement with mobile vertices.
            for (face_index, &(ids, rest_area)) in self.hand_faces.iter().enumerate() {
                let mut p = ids.map(|i| Vec3::from_array(vertices[i].position));
                if let Some(normal) = face_normals[face_index] {
                    for _ in 0..2 {
                        let signed = (p[1] - p[0]).cross(p[2] - p[0]).dot(normal);
                        if signed >= rest_area * 0.02 {
                            break;
                        }
                        let gradients = [
                            (p[1] - p[2]).cross(normal),
                            (p[2] - p[0]).cross(normal),
                            (p[0] - p[1]).cross(normal),
                        ];
                        let denominator: f32 = (0..3)
                            .map(|j| self.hand_mobility[ids[j]] * gradients[j].length_squared())
                            .sum();
                        if denominator <= 1e-14 {
                            break;
                        }
                        let correction = (rest_area * 0.02 - signed) / denominator;
                        for j in 0..3 {
                            p[j] += gradients[j] * correction * self.hand_mobility[ids[j]];
                            vertices[ids[j]].position = p[j].to_array();
                        }
                    }
                }
                let cross = (p[1] - p[0]).cross(p[2] - p[0]);
                let area = cross.length();
                let minimum = rest_area * 0.60;
                if area >= minimum || area < 1e-12 {
                    continue;
                }
                let normal = cross / area;
                let gradients = [
                    (p[1] - p[2]).cross(normal),
                    (p[2] - p[0]).cross(normal),
                    (p[0] - p[1]).cross(normal),
                ];
                let denominator: f32 = (0..3)
                    .map(|j| self.hand_mobility[ids[j]] * gradients[j].length_squared())
                    .sum();
                if denominator <= 1e-14 {
                    continue;
                }
                let correction = 0.5 * (minimum - area) / denominator;
                for j in 0..3 {
                    vertices[ids[j]].position =
                        (p[j] + gradients[j] * correction * self.hand_mobility[ids[j]]).to_array();
                }
            }
        }
    }
    pub(crate) fn grasp_amount(&self, time: f32) -> f32 {
        if self.grasp_cycle {
            gesture_envelope(time.rem_euclid(6.) - 0.20)
        } else {
            self.grasp_mix
        }
    }
    fn pose(&self, time: f32) -> Pose {
        let body_time = if self.body_motion { time } else { 0. };
        let relaxed = self.clip.sample(&self.skeleton, body_time);
        let amount = self.grasp_amount(time);
        if amount == 0. {
            return relaxed;
        }
        let mut posed = Pose::blend(
            &relaxed,
            &self.grasp_clip.sample(&self.skeleton, body_time),
            amount,
        )
        .expect("validated grasp");
        if let Some(samples) = &self.grasp_samples {
            let position = amount * 120.;
            let lower = (position as usize).min(119);
            let blend = position - lower as f32;
            for side in 0..2 {
                for finger in 0..5 {
                    for joint in 0..3 {
                        let q = samples[lower][finger][joint]
                            .slerp(samples[lower + 1][finger][joint], blend);
                        let q = if side == 0 {
                            q
                        } else {
                            Quat::from_xyzw(q.x, -q.y, -q.z, q.w)
                        };
                        posed
                            .set_joint_rotation(24 + side * 15 + finger * 3 + joint, q)
                            .expect("valid contact pose");
                    }
                }
            }
        }
        posed
    }
    pub(crate) fn hand_matrix(&self, time: f32, right: bool) -> Mat4 {
        self.pose(time)
            .skin_matrices(&self.skeleton)
            .expect("valid hand pose")[if right { 12 } else { 6 }]
    }
    pub fn head_matrix(&self, time: f32) -> Mat4 {
        self.pose(time)
            .skin_matrices(&self.skeleton)
            .expect("validated rig")[3]
    }
    /// Apply the same procedural skeleton to reduced physical-shell points.
    pub(crate) fn pose_points(&self, points: &[[f64; 3]], time: f32) -> Vec<[f64; 3]> {
        let mut vertices: Vec<_> = points
            .iter()
            .map(|p| SceneVertex {
                position: p.map(|v| v as f32),
                uv: [0.; 2],
                color: [1.; 4],
            })
            .collect();
        self.deform(&mut vertices, time);
        vertices
            .into_iter()
            .map(|v| v.position.map(f64::from))
            .collect()
    }
    fn inward_cylinder(&self) -> bool {
        matches!(
            self.grasp_object.as_deref(),
            Some(GraspObject::Cylinder { .. })
        ) && self
            .grasp_samples
            .as_ref()
            .is_some_and(|samples| samples[120][0][1].y < 0.)
    }
    pub fn deform(&self, vertices: &mut [SceneVertex], time: f32) {
        let hand_sides: Vec<_> = if self.inward_cylinder() {
            vertices
                .iter()
                .map(|vertex| vertex.position[0] < 0.)
                .collect()
        } else {
            Vec::new()
        };
        let palette = self
            .pose(time)
            .skin_matrices(&self.skeleton)
            .expect("validated rig");
        let left_hand_inverse = palette[6].inverse();
        let right_hand_inverse = palette[12].inverse();
        let finger_palette: Vec<_> = palette
            .iter()
            .enumerate()
            .map(|(index, matrix)| {
                crate::rig_skinning::RigidSkinTransform::from_matrix(if index < 16 {
                    Mat4::IDENTITY
                } else {
                    (if (16..20).contains(&index) || (24..39).contains(&index) {
                        left_hand_inverse
                    } else {
                        right_hand_inverse
                    }) * *matrix
                })
            })
            .collect();
        let palette: Vec<_> = palette
            .into_iter()
            .map(crate::rig_skinning::RigidSkinTransform::from_matrix)
            .collect();
        assert_eq!(
            vertices.len(),
            self.weights.len(),
            "skin binding count mismatch"
        );
        let face_normals: Vec<_> = self
            .hand_faces
            .iter()
            .map(|(ids, _)| {
                let p = ids.map(|i| Vec3::from_array(vertices[i].position));
                let center = (p[0] + p[1] + p[2]) / 3.;
                if !(0.33..0.37).contains(&center.x.abs())
                    || !(-0.025..0.025).contains(&center.y)
                    || !(0.070..0.115).contains(&center.z)
                {
                    return None;
                }
                let normal = (p[1] - p[0]).cross(p[2] - p[0]).normalize();
                let rotated = ids.map(|i| {
                    let local = if let Some(fingers) = &self.finger_weights[i] {
                        crate::rig_skinning::deform_point(normal, fingers, &finger_palette)
                            - crate::rig_skinning::deform_point(
                                Vec3::ZERO,
                                fingers,
                                &finger_palette,
                            )
                    } else {
                        normal
                    };
                    crate::rig_skinning::deform_point(local, &self.weights[i], &palette)
                        - crate::rig_skinning::deform_point(Vec3::ZERO, &self.weights[i], &palette)
                });
                (rotated[0] + rotated[1] + rotated[2]).try_normalize()
            })
            .collect();
        for ((v, weights), fingers) in vertices
            .iter_mut()
            .zip(&self.weights)
            .zip(&self.finger_weights)
        {
            let mut point = Vec3::from_array(v.position);
            if let Some(fingers) = fingers {
                point = crate::rig_skinning::deform_point(point, fingers, &finger_palette);
            }
            v.position = crate::rig_skinning::deform_point(point, weights, &palette).to_array();
        }
        let closure = if self.grasp_cycle {
            gesture_envelope(time.rem_euclid(6.) - 0.20)
        } else {
            self.grasp_mix
        };
        if closure > 0. {
            self.correct_grasp_skin(vertices, &face_normals);
            if !hand_sides.is_empty() {
                let object = self.grasp_object.as_ref().unwrap();
                let hands = [self.hand_matrix(time, false), self.hand_matrix(time, true)];
                let inverse = hands.map(|matrix| matrix.inverse());
                for (index, vertex) in vertices.iter_mut().enumerate() {
                    if self.hand_mobility.get(index).copied().unwrap_or(0.) == 0. {
                        continue;
                    }
                    let side = usize::from(hand_sides[index]);
                    let mirror = Vec3::new(if side == 1 { -1. } else { 1. }, 1., 1.);
                    let mut point =
                        inverse[side].transform_point3(Vec3::from_array(vertex.position)) * mirror;
                    for _ in 0..3 {
                        let gap = object.distance(point);
                        if gap >= 0.000001 {
                            break;
                        }
                        let gradient = Vec3::from_array(std::array::from_fn(|axis| {
                            let mut plus = point;
                            let mut minus = point;
                            plus[axis] += 0.00001;
                            minus[axis] -= 0.00001;
                            (object.distance(plus) - object.distance(minus)) / 0.00002
                        }));
                        point += gradient.normalize_or_zero() * (0.000001 - gap);
                    }
                    vertex.position = hands[side].transform_point3(point * mirror).to_array();
                }
            }
        }
    }
}

/// Rest, raise, settle, return: quintic easing has zero velocity and acceleration
/// at each hold boundary, unlike a sampled linear ramp or mechanical sine sway.
fn gesture_envelope(time: f32) -> f32 {
    fn ease(value: f32) -> f32 {
        let t = value.clamp(0., 1.);
        t * t * t * (t * (t * 6. - 15.) + 10.)
    }
    if time < 0.5 {
        0.
    } else if time < 2.5 {
        ease((time - 0.5) / 2.)
    } else if time < 3.0 {
        1.
    } else if time < 5.5 {
        1. - ease((time - 3.) / 2.5)
    } else {
        0.
    }
}

fn smooth(a: f32, b: f32, value: f32) -> f32 {
    let t = ((value - a) / (b - a)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}
/// Restrict weights to anatomically connected regions: a nearby hand cannot pull the hip,
/// and the opposite arm cannot influence this arm. Joint bands blend adjacent bones only.
// Imported mannequin finger centerlines, in the same metre/Y-up rest space as body.obj.
// Thumb plus index, middle, ring and little finger; three articulated segments each.
const FINGER_CHAINS: [[[f32; 3]; 4]; 5] = [
    [
        [0.347, 0.009, 0.080],
        // Length-weighted centers of body.obj section contours at these joints.
        [0.346454, -0.003967, 0.105662],
        [0.350558, -0.019856, 0.125033],
        [0.353366, -0.029253, 0.138596],
    ],
    [
        [0.389, -0.043, 0.097],
        [0.402131, -0.062528, 0.112442],
        [0.403708, -0.082709, 0.124597],
        [0.407805, -0.098480, 0.133655],
    ],
    [
        [0.391, -0.047, 0.069],
        [0.409516, -0.072594, 0.083647],
        [0.413729, -0.097898, 0.095407],
        [0.417222, -0.121835, 0.104786],
    ],
    [
        [0.386, -0.047, 0.043],
        [0.405946, -0.074401, 0.055381],
        [0.409063, -0.099984, 0.064022],
        [0.410703, -0.124835, 0.073525],
    ],
    [
        [0.377, -0.052, 0.022],
        [0.396234, -0.076375, 0.023712],
        [0.400225, -0.095935, 0.025124],
        [0.404223, -0.113924, 0.027238],
    ],
];
/// Rest-space rotational axes. Positive flexion moves the distal direction toward
/// the palmar side (-X on the source left hand). These are axial vectors: their
/// mirror is (x, -y, -z), while joint positions mirror as (-x, y, z).
#[derive(Clone, Copy, Debug)]
struct FingerJointAxes {
    flexion: Vec3,
    spread: Vec3,
    twist: Vec3,
}
impl FingerJointAxes {
    fn for_segment(chain: [[f32; 3]; 4], segment: usize, side: f32) -> Self {
        debug_assert!(side == 1. || side == -1.);
        let mirror = |p: [f32; 3]| Vec3::new(side * p[0], p[1], p[2]);
        let direction = (mirror(chain[segment + 1]) - mirror(chain[segment])).normalize();
        let palm = Vec3::new(-side, 0., 0.);
        let flexion = direction.cross(palm).normalize();
        // A longitudinal rotation axis must also mirror as an axial vector.
        let twist = direction * side;
        let spread = twist.cross(flexion).normalize();
        Self {
            flexion,
            spread,
            twist,
        }
    }

    /// Fixed rest-axis control order: flexion, then spread, then longitudinal twist.
    /// All inputs are radians. Thumb opposition requires spread/twist of its
    /// metacarpal; it must not be encoded by reversing phalanx flexion signs.
    fn rotation(self, flexion: f32, spread: f32, twist: f32) -> Quat {
        Quat::from_axis_angle(self.twist, twist)
            * Quat::from_axis_angle(self.spread, spread)
            * Quat::from_axis_angle(self.flexion, flexion)
    }
}
fn finger_clearance(chain: [[f32; 3]; 4], rotations: [Quat; 3], object: &GraspObject) -> f32 {
    finger_clearance_from(chain, rotations, object, 0)
}
fn finger_clearance_from(
    chain: [[f32; 3]; 4],
    rotations: [Quat; 3],
    object: &GraspObject,
    first: usize,
) -> f32 {
    if let Some(finger) = FINGER_CHAINS
        .iter()
        .position(|candidate| *candidate == chain)
    {
        if finger != 0 || !matches!(object, GraspObject::Cylinder { .. }) {
            // Use both actual hand surfaces: their vertices and skin weights differ.
            // Retain a small clearance margin before the final surface correction.
            type Cloud = Vec<(Vec3, Box<[(usize, f32)]>)>;
            static CLOUDS: std::sync::OnceLock<[[Cloud; 3]; 5]> = std::sync::OnceLock::new();
            let clouds = CLOUDS.get_or_init(|| {
                let asset = voxy_render::ObjAsset::parse(
                    include_str!("/Users/themoretheless/Documents/ChatGPT/Voxy/assets/characters/blender-female/body.obj"),
                    voxy_render::ObjLimits::default(),
                )
                .unwrap();
                let mut clouds: [[Cloud; 3]; 5] =
                    std::array::from_fn(|_| std::array::from_fn(|_| Vec::new()));
                for vertex in asset.mesh.vertices() {
                    let point = Vec3::from_array(vertex.position);
                    if point.x.abs() <= 0.32 {
                        continue;
                    }
                    let side = usize::from(point.x < 0.);
                    let canonical = Vec3::new(point.x.abs(), point.y, point.z);
                    let Some(weights) = finger_weights(point) else {
                        continue;
                    };
                    for finger in 0..5 {
                        let root = 24 + side * 15 + finger * 3;
                        let own: f32 = weights
                            .iter()
                            .filter(|(bone, _)| (root..root + 3).contains(bone))
                            .map(|(_, weight)| weight)
                            .sum();
                        if own < 0.8 {
                            continue;
                        }
                        let mut compact: Vec<_> = weights
                            .iter()
                            .filter(|(bone, weight)| {
                                (root..root + 3).contains(bone) && *weight > 0.
                            })
                            .map(|(bone, weight)| (*bone - side * 15, *weight))
                            .collect();
                        compact.push((0, 1. - own));
                        for first in 0..3 {
                            if compact.iter().any(|(bone, weight)| {
                                (24 + finger * 3 + first..24 + finger * 3 + 3).contains(bone)
                                    && *weight > 0.
                            }) {
                                clouds[finger][first]
                                    .push((canonical, compact.clone().into_boxed_slice()));
                            }
                        }
                    }
                }
                clouds
            });
            if !clouds[finger][first].is_empty() {
                return finger_surface_gap(
                    chain,
                    finger,
                    rotations,
                    object,
                    &clouds[finger][first],
                ) - if finger == 0 { 0.0002 } else { 0.0001 };
            }
        }
    }
    let mut point = Vec3::from_array(chain[0]);
    let mut rotation = Quat::IDENTITY;
    let mut clearance = f32::INFINITY;
    for segment in 0..3 {
        rotation *= rotations[segment];
        let delta =
            rotation * (Vec3::from_array(chain[segment + 1]) - Vec3::from_array(chain[segment]));
        let rest_direction =
            (Vec3::from_array(chain[segment + 1]) - Vec3::from_array(chain[segment])).normalize();
        let transverse = (Vec3::X - rest_direction * rest_direction.x).normalize();
        let lateral = rest_direction.cross(transverse);
        for sample in 0..=64 {
            if segment < first {
                continue;
            }
            let fraction = sample as f32 / 64.;
            let center = point + delta * fraction;
            let radius = if chain == FINGER_CHAINS[0] {
                // Thumb sections are broad across X and taper toward the tip.
                // Use their measured section widths rather than a circular proxy.
                let width = [0.009, 0.0092, 0.0100, 0.0074];
                let depth = [0.0085, 0.0096, 0.0074, 0.0053];
                let a = width[segment] * (1. - fraction) + width[segment + 1] * fraction;
                let b = depth[segment] * (1. - fraction) + depth[segment + 1] * fraction;
                let normal = rotation.conjugate()
                    * (object.closest_point(center) - center)
                        .try_normalize()
                        .unwrap_or(Vec3::X);
                ((normal.dot(transverse) * a).powi(2)
                    + (normal.dot(lateral) * b).powi(2)
                    + (normal.dot(rest_direction) * 0.005).powi(2))
                .sqrt()
            } else if segment == 2 {
                // Distal sections taper, especially on the little finger. These
                // half-widths enclose the imported mesh's measured section contours.
                let finger = FINGER_CHAINS
                    .iter()
                    .position(|candidate| *candidate == chain)
                    .unwrap_or(1);
                let transverse_tip = [0.00675, 0.0064, 0.0065, 0.0060, 0.0054][finger];
                let lateral_tip = [0.00675, 0.0068, 0.0075, 0.0068, 0.0060][finger];
                let a = 0.00675 * (1. - fraction) + transverse_tip * fraction;
                let b = 0.00675 * (1. - fraction) + lateral_tip * fraction;
                let normal = rotation.conjugate()
                    * (object.closest_point(center) - center)
                        .try_normalize()
                        .unwrap_or(Vec3::X);
                // The actual tips extend only 1.5–2.2 mm past the last joint.
                let tip_length = [0.00675, 0.0021, 0.0023, 0.0016, 0.0020][finger];
                let longitudinal = 0.00675 * (1. - fraction) + tip_length * fraction;
                ((normal.dot(transverse) * a).powi(2)
                    + (normal.dot(lateral) * b).powi(2)
                    + (normal.dot(rest_direction) * longitudinal).powi(2))
                .sqrt()
            } else {
                0.00675
            };
            // Keep the measured thumb envelope conservative while refining other fingers.
            let sampling_margin =
                delta.length() / if chain == FINGER_CHAINS[0] { 32. } else { 128. };
            clearance = clearance.min(object.distance(center) - radius - sampling_margin);
        }
        point += delta;
    }
    clearance
}
/// Use the same bounded, collision-constrained path for target selection and playback.
fn prepare_finger_grasp(
    chain: [[f32; 3]; 4],
    full: [Quat; 3],
    object: &GraspObject,
) -> Vec<[Quat; 3]> {
    let mut previous = [Quat::IDENTITY; 3];
    (0..=120)
        .map(|frame| {
            let desired = full.map(|rotation| Quat::IDENTITY.slerp(rotation, frame as f32 / 120.));
            let goal = fit_finger(chain, previous, desired, object);
            let target = std::array::from_fn(|joint| {
                let from = previous[joint];
                let angle = from.angle_between(goal[joint]);
                from.slerp(goal[joint], if angle > 0.012 { 0.012 / angle } else { 1. })
            });
            previous = fit_finger(chain, previous, target, object);
            previous
        })
        .collect()
}

fn fit_finger(
    chain: [[f32; 3]; 4],
    open: [Quat; 3],
    desired: [Quat; 3],
    object: &GraspObject,
) -> [Quat; 3] {
    let mut fitted = open;
    // An object already intersecting the open finger cannot be fixed by closing it.
    if finger_clearance(chain, open, object) < -1e-6 {
        return open;
    }
    // Advance the whole finger together until first contact. Otherwise a distal
    // joint can consume all clearance before the knuckle starts to articulate.
    let mut safe = 0.;
    for step in 1..=32 {
        let fraction = step as f32 / 32.;
        let candidate = std::array::from_fn(|joint| open[joint].slerp(desired[joint], fraction));
        if finger_clearance(chain, candidate, object) < 0. {
            let mut blocked = fraction;
            for _ in 0..12 {
                let middle = (safe + blocked) * 0.5;
                let candidate =
                    std::array::from_fn(|joint| open[joint].slerp(desired[joint], middle));
                if finger_clearance(chain, candidate, object) >= 0. {
                    safe = middle;
                } else {
                    blocked = middle;
                }
            }
            safe = (safe - 1e-5).max(0.);
            fitted = std::array::from_fn(|joint| open[joint].slerp(desired[joint], safe));
            break;
        }
        safe = fraction;
        fitted = candidate;
    }
    let mut order = [0, 1, 2];
    // Release opening joints first; then curl unconstrained distal segments.
    order.sort_by_key(|&joint| {
        let opening = desired[joint].angle_between(Quat::IDENTITY)
            < open[joint].angle_between(Quat::IDENTITY) - 1e-6;
        (
            usize::from(!opening),
            if opening { joint } else { 2 - joint },
        )
    });
    for joint in order {
        let from = fitted[joint];
        let mut safe = 0.;
        for step in 1..=32 {
            let fraction = step as f32 / 32.;
            fitted[joint] = from.slerp(desired[joint], fraction);
            if finger_clearance_from(chain, fitted, object, joint) < 0. {
                let mut blocked = fraction;
                for _ in 0..12 {
                    let middle = (safe + blocked) * 0.5;
                    fitted[joint] = from.slerp(desired[joint], middle);
                    if finger_clearance_from(chain, fitted, object, joint) >= 0. {
                        safe = middle;
                    } else {
                        blocked = middle;
                    }
                }
                fitted[joint] = from.slerp(desired[joint], (safe - 1e-5).max(0.));
                break;
            }
            safe = fraction;
        }
    }
    // Find a coupled slide only for the reference target. Playback still moves
    // toward this target through the existing bounded, swept contact path.
    if chain != FINGER_CHAINS[0] {
        let axes: [Vec3; 3] =
            std::array::from_fn(|joint| FingerJointAxes::for_segment(chain, joint, 1.).flexion);
        // Advance the fingertip toward the opposed side of the canonical hand.
        // A sum of joint angles can increase curl while moving away from the object.
        let score = |pose: [Quat; 3]| {
            let mut point = Vec3::from_array(chain[0]);
            let mut rotation = Quat::IDENTITY;
            for joint in 0..3 {
                rotation *= pose[joint];
                point += rotation
                    * (Vec3::from_array(chain[joint + 1]) - Vec3::from_array(chain[joint]));
            }
            -point.x
        };
        for _ in 0..8 {
            let mut best = fitted;
            let mut best_score = score(fitted);
            for joint in [1, 2] {
                // Resolve the contact boundary continuously: fixed angle ratios
                // can miss the narrow set of poses that both slide and touch.
                let mut opened = fitted;
                opened[joint] *= Quat::from_axis_angle(axes[joint], -0.012);
                if Vec3::new(opened[joint].x, opened[joint].y, opened[joint].z).dot(axes[joint])
                    >= 0.
                    && finger_clearance(chain, opened, object) >= 0.
                {
                    let at = |angle: f32| {
                        let mut pose = opened;
                        pose[0] *= Quat::from_axis_angle(axes[0], angle);
                        pose
                    };
                    let mut lower = 0.;
                    let mut upper = None;
                    for step in 1..=12 {
                        let angle = step as f32 * 0.002;
                        let pose = at(angle);
                        if pose[0].angle_between(Quat::IDENTITY)
                            > desired[0].angle_between(Quat::IDENTITY)
                        {
                            break;
                        }
                        if finger_clearance(chain, pose, object) < 0. {
                            upper = Some(angle);
                            break;
                        }
                        lower = angle;
                    }
                    if let Some(mut upper) = upper {
                        for _ in 0..16 {
                            let middle = (lower + upper) * 0.5;
                            if finger_clearance(chain, at(middle), object) >= 0. {
                                lower = middle;
                            } else {
                                upper = middle;
                            }
                        }
                    }
                    let candidate = at(lower);
                    if open.iter().any(|q| *q != Quat::IDENTITY)
                        && (0..3).any(|j| open[j].angle_between(candidate[j]) > 0.012)
                    {
                        continue;
                    }
                    let candidate_score = score(candidate);
                    if candidate_score > best_score
                        && finger_clearance(chain, candidate, object) <= 0.00001
                        && (1..=8).all(|step| {
                            let pose = std::array::from_fn(|i| {
                                fitted[i].slerp(candidate[i], step as f32 / 8.)
                            });
                            finger_clearance(chain, pose, object) >= 0.
                        })
                    {
                        best = candidate;
                        best_score = candidate_score;
                    }
                }
                for (direction, ratio) in [-1., 1.]
                    .into_iter()
                    .flat_map(|direction| [0.5, 1., 1.5, 2., 3.].map(|ratio| (direction, ratio)))
                {
                    let mut candidate = fitted;
                    candidate[0] *= Quat::from_axis_angle(axes[0], direction * 0.004);
                    candidate[joint] *=
                        Quat::from_axis_angle(axes[joint], -direction * 0.004 * ratio);
                    let root_flex =
                        Vec3::new(candidate[0].x, candidate[0].y, candidate[0].z).dot(axes[0]);
                    if root_flex < 0.07
                        || Vec3::new(candidate[joint].x, candidate[joint].y, candidate[joint].z)
                            .dot(axes[joint])
                            < 0.
                        || candidate[0].angle_between(Quat::IDENTITY)
                            > desired[0].angle_between(Quat::IDENTITY)
                        || candidate[joint].angle_between(Quat::IDENTITY)
                            > desired[joint].angle_between(Quat::IDENTITY)
                    {
                        continue;
                    }
                    if open.iter().any(|q| *q != Quat::IDENTITY)
                        && (0..3).any(|j| open[j].angle_between(candidate[j]) > 0.012)
                    {
                        continue;
                    }
                    let candidate_score = score(candidate);
                    if candidate_score <= best_score {
                        continue;
                    }
                    // Sliding must retain contact, rather than maximizing curl
                    // by withdrawing the fingertip from the object's surface.
                    if finger_clearance(chain, candidate, object) > 0.00001 {
                        continue;
                    }
                    if (1..=4).all(|step| {
                        let pose = std::array::from_fn(|i| {
                            fitted[i].slerp(candidate[i], step as f32 / 4.)
                        });
                        finger_clearance(chain, pose, object) >= 0.
                    }) {
                        best = candidate;
                        best_score = candidate_score;
                    }
                }
            }
            if best == fitted {
                break;
            }
            fitted = best;
        }
    }
    fitted
}

fn curled_thumb_contact_path(
    reference: &[[Quat; 3]],
    object: &GraspObject,
    skin: &[(Vec3, Box<[(usize, f32)]>)],
) -> Option<Vec<[Quat; 3]>> {
    let axes: [Vec3; 3] = std::array::from_fn(|j| {
        FingerJointAxes::for_segment(FINGER_CHAINS[0], j, 1.).flexion
    });
    // Require contact on the distal thumb surface, rather than any phalanx.
    let pad: Vec<_> = skin
        .iter()
        .filter(|(_, weights)| weights.iter().any(|(bone, w)| *bone == 26 && *w >= 0.80))
        .cloned()
        .collect();
    if pad.is_empty() {
        return None;
    }
    let mut candidates = Vec::new();
    let mesh = matches!(object, GraspObject::Mesh { .. });
    let cylinder=matches!(object,GraspObject::Cylinder{..});
    let x_limit = if mesh || cylinder { 40 } else { 25 };
    let y_limit = if cylinder {35} else if mesh {30} else {15};
    for x in -x_limit..=x_limit {
        for y in -y_limit..=y_limit {
            let root = Quat::from_rotation_y(y as f32 * 0.02)
                * Quat::from_rotation_x(x as f32 * 0.02)
                * reference[120][0];
            if root.angle_between(Quat::IDENTITY)>1.10 {continue;}
            let pose = [
                root,
                Quat::from_axis_angle(axes[1], if cylinder {0.65} else if mesh {0.35} else {0.50}),
                Quat::from_axis_angle(axes[2], if cylinder {0.45} else if mesh {0.22} else {0.30}),
            ];
            let gap = finger_clearance(FINGER_CHAINS[0], pose, object);
            if gap.abs() < 0.0008 {
                candidates.push((root.angle_between(Quat::IDENTITY), pose));
            }
        }
    }
    candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (_, pose) in candidates {
        if let Some(path) = projected_thumb_contact_path(pose, object, skin, 0.0124) {
            let gap = finger_surface_gap(FINGER_CHAINS[0], 0, path[120], object, &pad);
            if !(0. ..=0.002).contains(&gap) {
                continue;
            }
            return Some(path);
        }
    }
    None
}

fn cylinder_thumb_contact_path(reference:&[[Quat;3]],object:&GraspObject,skin:&[(Vec3,Box<[(usize,f32)]>)])->Option<Vec<[Quat;3]>> {
curled_thumb_contact_path(reference,object,skin)
}

fn projected_thumb_contact_path(
    mut target: [Quat; 3],
    object: &GraspObject,
    skin: &[(Vec3, Box<[(usize, f32)]>)],
    maximum_step: f32,
) -> Option<Vec<[Quat; 3]>> {
    // Project the opposed root onto first contact while retaining the curled phalanges.
    let original = target[0];
    let contact_margin = 0.000002;
    let inside = finger_clearance(FINGER_CHAINS[0], target, object) < contact_margin;
    let other = [-0.01, 0.01].into_iter().find_map(|angle| {
        let q = Quat::from_rotation_y(angle) * original;
        target[0] = q;
        ((finger_clearance(FINGER_CHAINS[0], target, object) < contact_margin) != inside)
            .then_some(q)
    })?;
    let (safe, blocked) = if inside {
        (other, original)
    } else {
        (original, other)
    };
    let mut low = 0.;
    let mut high = 1.;
    for _ in 0..16 {
        let mid = (low + high) * 0.5;
        target[0] = safe.slerp(blocked, mid);
        if finger_clearance(FINGER_CHAINS[0], target, object) >= contact_margin {
            low = mid;
        } else {
            high = mid;
        }
    }
    target[0] = safe.slerp(blocked, (low - 0.0001).max(0.));
    let mut frames: [usize; 3] =
        target.map(|q| (q.angle_between(Quat::IDENTITY) / maximum_step).ceil() as usize);
    frames[1] = frames[1].max(frames[2]);
    frames[2] = frames[1];
    if frames.iter().any(|&n| n > 120) {
        return None;
    }
    let path: Vec<[Quat; 3]> = (0..=120)
        .map(|frame| {
            std::array::from_fn(|j| {
                Quat::IDENTITY.slerp(target[j], (frame as f32 / frames[j].max(1) as f32).min(1.))
            })
        })
        .collect();
    if path.iter().any(|&q| {
        finger_clearance(FINGER_CHAINS[0], q, object) < 0.
            || finger_surface_gap(FINGER_CHAINS[0], 0, q, object, skin) < 0.
    }) {
        return None;
    }
    let gap = finger_surface_gap(FINGER_CHAINS[0], 0, target, object, skin);
    if !(0. ..=0.002).contains(&gap) {
        return None;
    }
    Some(path)
}

fn finger_surface_gap(
    chain: [[f32; 3]; 4],
    finger: usize,
    pose: [Quat; 3],
    object: &GraspObject,
    skin: &[(Vec3, Box<[(usize, f32)]>)],
) -> f32 {
    let mut palette =
        vec![crate::rig_skinning::RigidSkinTransform::from_matrix(Mat4::IDENTITY); 54];
    let mut transform = Mat4::IDENTITY;
    for joint in 0..3 {
        let origin = Vec3::from_array(chain[joint]);
        let parent = if joint == 0 {
            Vec3::ZERO
        } else {
            Vec3::from_array(chain[joint - 1])
        };
        transform *= Mat4::from_translation(origin - parent) * Mat4::from_quat(pose[joint]);
        palette[24 + finger * 3 + joint] = crate::rig_skinning::RigidSkinTransform::from_matrix(
            transform * Mat4::from_translation(-origin),
        );
    }
    let skin_gap = skin
        .iter()
        .map(|(point, weights)| {
            object.distance(crate::rig_skinning::deform_point(*point, weights, &palette))
        })
        .fold(f32::INFINITY, f32::min);
    skin_gap
}

fn cylinder_curled_contact_pose(
    chain: [[f32; 3]; 4],
    finger: usize,
    object: &GraspObject,
    skin: &[(Vec3, Box<[(usize, f32)]>)],
    adduction: Quat,
) -> Option<[Quat; 3]> {
    let GraspObject::Cylinder { center, .. } = object else {
        return None;
    };
    let axes: [Vec3; 3] =
        std::array::from_fn(|j| FingerJointAxes::for_segment(chain, j, 1.).flexion);
    let pad: Vec<_> = skin
        .iter()
        .filter(|(_, w)| {
            w.iter()
                .any(|(bone, weight)| *bone == 26 + finger * 3 && *weight >= 0.8)
        })
        .cloned()
        .collect();
    if pad.is_empty() {
        return None;
    }
    let mut best = None;
    let mut best_score = -f32::INFINITY;
    for a in 3..=28 {
        for b in 3..=30 {
            for c in 0..=24 {
                let angles = [a as f32 * 0.05, b as f32 * 0.05, c as f32 * 0.05];
                let pose = [
                    adduction * Quat::from_axis_angle(axes[0], angles[0]),
                    Quat::from_axis_angle(axes[1], angles[1]),
                    Quat::from_axis_angle(axes[2], angles[2]),
                ];
                let mut point = Vec3::from_array(chain[0]);
                let mut rotation = Quat::IDENTITY;
                let mut centers_safe = true;
                for j in 0..3 {
                    rotation *= pose[j];
                    point +=
                        rotation * (Vec3::from_array(chain[j + 1]) - Vec3::from_array(chain[j]));
                    if object.distance(point) < 0.003 {
                        centers_safe = false;
                        break;
                    }
                }
                if !centers_safe || point.y >= center.y {
                    continue;
                }
                let score = -(point.y - center.y).atan2(point.x - center.x);
                if score < best_score {
                    continue;
                }
                let gap = finger_clearance(chain, pose, object);
                if !(0. ..=0.002).contains(&gap) {
                    continue;
                }
                let pad_gap = finger_surface_gap(chain, finger, pose, object, &pad);
                if !(0. ..=0.002).contains(&pad_gap) {
                    continue;
                }
                best_score = score;
                best = Some(pose);
            }
        }
    }
    let best = best?;
    let mut toward = best;
    toward[0] *= Quat::from_axis_angle(axes[0], 0.05);
    let target = fit_finger(chain, best, toward, object);
    Some(target)
}

fn grasp_rotation(finger:usize,segment:usize,sign:f32)->Quat {
let axes=FingerJointAxes::for_segment(FINGER_CHAINS[finger],segment,sign);
let flex=if finger==0{[0.20,0.65,0.45]}else{[0.60,0.95,0.55]};
axes.rotation(flex[segment],if finger==0 && segment==0{0.70}else{0.},if finger==0 && segment==0{0.20}else{0.})
}
fn finger_weights(p:Vec3)->Option<Vec<(usize,f32)>> {
static BIND:std::sync::OnceLock<std::collections::BTreeMap<[i32;3],[f32;54]>>=std::sync::OnceLock::new();
let bind=BIND.get_or_init(||include_str!("/Users/themoretheless/Documents/ChatGPT/Voxy/target/hand-metacarpal-engine-audit/weights-conditioned.csv").lines().map(|line|{let v:Vec<f32>=line.split(',').map(|x|x.parse().unwrap()).collect();(std::array::from_fn(|i|(v[i]*1_000_000.).round() as i32),std::array::from_fn(|i|v[i+3]))}).collect());
let key=p.to_array().map(|v|(v*1_000_000.).round()as i32);
if let Some(w)=bind.get(&key){return Some(w.iter().enumerate().filter_map(|(i,&w)|(w>0.).then_some((i,w))).collect());}
finger_weights_uncorrected(p)
}
fn finger_weights_uncorrected(p: Vec3) -> Option<Vec<(usize, f32)>> {
    let point = Vec3::new(p.x.abs(), p.y, p.z);
    if point.x < 0.32 || point.y > 0.03 || point.y < -0.16 {
        return None;
    }
    let mut samples = [(0_f32, 0_f32); 5];
    for (finger, chain) in FINGER_CHAINS.iter().enumerate() {
        let mut prefix = 0.;
        let mut best = (f32::INFINITY, 0.);
        let mut segments = [(0_f32, 0_f32); 3];
        for segment in 0..3 {
            let a = Vec3::from_array(chain[segment]);
            let b = Vec3::from_array(chain[segment + 1]);
            let delta = b - a;
            let t = ((point - a).dot(delta) / delta.length_squared()).clamp(0., 1.);
            let distance = point.distance_squared(a + t * delta);
            segments[segment] = (distance, prefix + t * delta.length());
            if distance < best.0 {
                best = (distance, prefix + t * delta.length());
            }
            prefix += delta.length();
        }
        if finger == 0 {
            let weights =
                segments.map(|sample| (-(sample.0 - best.0) / (2. * 0.004_f32.powi(2))).exp());
            let soft_arc = (0..3).map(|i| weights[i] * segments[i].1).sum::<f32>()
                / weights.iter().sum::<f32>();
            let blend = 1. - smooth(0.008, 0.016, (point.x - chain[0][0]).abs());
            best.1 += (soft_arc - best.1) * blend;
        }
        samples[finger] = best;
    }
    // Smooth assignment across the shared webbing avoids a discontinuous nearest-finger
    // boundary. Keep every contribution here; pruning to four weights reintroduces seams.
    let minimum = samples
        .iter()
        .map(|sample| sample.0)
        .fold(f32::INFINITY, f32::min);
    let sigma = 0.006 + 0.003 * smooth(-0.050, -0.010, point.y) * smooth(0.060, 0.090, point.z);
    let probabilities = samples.map(|sample| (-(sample.0 - minimum) / (2. * sigma.powi(2))).exp());
    let total: f32 = probabilities.iter().sum();
    let side = if p.x >= 0. { 0 } else { 1 };
    let envelope = smooth(0.32, 0.34, point.x) * (1. - smooth(0.015, 0.03, point.y));
    let mut result = Vec::with_capacity(19);
    let mut sum = 0.;
    for finger in 0..5 {
        let arc = samples[finger].1;
        let chain = FINGER_CHAINS[finger].map(Vec3::from_array);
        let first = chain[0].distance(chain[1]);
        let second = first + chain[1].distance(chain[2]);
        let root_blend = if finger == 0 {
            // Follow the thumb axis instead of cutting across its web in world Z.
            (arc / 0.036).clamp(0., 1.)
        } else {
            smooth(0., 0.025, chain[0].y - point.y)
        };
        let influence = envelope * probabilities[finger] / total * root_blend;
        sum += influence;
        let middle_width = if finger == 0 { 0.012 } else { 0.014 };
        let middle = smooth(first - middle_width, first + middle_width, arc);
        let distal_width = if finger == 0 { 0.008 } else { 0.010 };
        let distal = smooth(second - distal_width, second + distal_width, arc);
        let start = 24 + side * 15 + finger * 3;
        result.extend([
            (start, influence * (1. - middle)),
            (start + 1, influence * middle * (1. - distal)),
            (start + 2, influence * middle * distal),
        ]);
    }
    if sum == 0. {
        return None;
    }
    result.push((0, 1. - sum));
    Some(result)
}
fn anatomical_weights(p: Vec3) -> [(usize, f32); 4] {
    if p.y > 0.62 {
        return [(3, 1.), (0, 0.), (0, 0.), (0, 0.)];
    }
    let neck = smooth(0.46, 0.62, p.y) * (1. - smooth(0.08, 0.18, p.x.abs()));
    let arm_boundary = 0.135 + (0.43 - p.y).max(0.) * 0.10;
    let shoulder_band = 0.045 + 0.03 * smooth(0.28, 0.40, p.y);
    let arm = smooth(arm_boundary, arm_boundary + shoulder_band, p.x.abs())
        * smooth(-0.24, -0.16, p.y)
        * (1. - smooth(0.43, 0.58, p.y));
    let start = if p.x >= 0. { 4 } else { 10 };
    let leg = 1. - smooth(-0.18, -0.10, p.y);
    if leg > 0. && arm == 0. {
        let shin = 1. - smooth(-0.52, -0.36, p.y);
        let foot = 1. - smooth(-0.755, -0.70, p.y);
        return [
            (0, 1. - leg),
            (start + 3, leg * (1. - shin)),
            (start + 4, leg * shin * (1. - foot)),
            (start + 5, leg * shin * foot),
        ];
    }
    let forearm = 1. - smooth(0.08, 0.32, p.y);
    let hand = 1. - smooth(-0.045, 0.025, p.y);
    if arm > 0. {
        return [
            (2, 1. - arm),
            (start, arm * (1. - forearm)),
            (start + 1, arm * forearm * (1. - hand)),
            (start + 2, arm * forearm * hand),
        ];
    }
    let chest = smooth(0.16, 0.30, p.y);
    let spine = smooth(-0.02, 0.10, p.y);
    [
        (0, (1. - spine) * (1. - neck)),
        (1, spine * (1. - chest) * (1. - neck)),
        (2, chest * (1. - neck)),
        (3, neck),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tetra_vertices() -> Vec<SceneVertex> {
        [Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z]
            .into_iter()
            .map(|p| SceneVertex {
                position: p.to_array(),
                uv: [0.; 2],
                color: [1.; 4],
            })
            .collect()
    }
    const TETRA_FACES: [u32; 12] = [0, 2, 1, 0, 1, 3, 0, 3, 2, 1, 2, 3];
    #[test]
    fn imported_grasp_mesh_rejects_open_inverted_and_nonmanifold_surfaces() {
        let vertices = tetra_vertices();
        let object = GraspObject::from_mesh(&vertices, &TETRA_FACES).unwrap();
        assert!(object.distance(Vec3::splat(0.1)) < 0.);
        assert!(object.distance(Vec3::splat(1.)) > 0.);
        assert!(GraspObject::from_mesh(&vertices, &TETRA_FACES[3..]).is_err());
        let mut reversed = TETRA_FACES;
        for face in reversed.chunks_exact_mut(3) {
            face.swap(1, 2);
        }
        assert!(GraspObject::from_mesh(&vertices, &reversed).is_err());
        let mut inconsistent = TETRA_FACES;
        inconsistent.swap(1, 2);
        assert!(GraspObject::from_mesh(&vertices, &inconsistent).is_err());
        let mut duplicate = TETRA_FACES.to_vec();
        duplicate.extend_from_slice(&TETRA_FACES[..3]);
        assert!(GraspObject::from_mesh(&vertices, &duplicate).is_err());
        let mut degenerate = TETRA_FACES;
        degenerate[1] = degenerate[0];
        assert!(GraspObject::from_mesh(&vertices, &degenerate).is_err());
        let mut invalid = vertices.clone();
        invalid[0].position[0] = f32::NAN;
        assert!(GraspObject::from_mesh(&invalid, &TETRA_FACES).is_err());
    }
    #[test]
    fn imported_grasp_mesh_welds_position_seams_and_preserves_cavities() {
        let vertices = tetra_vertices();
        let expanded: Vec<_> = TETRA_FACES.iter().map(|&i| vertices[i as usize]).collect();
        let indices: Vec<_> = (0..12).collect();
        let welded = GraspObject::from_mesh(&expanded, &indices).unwrap();
        assert!(welded.distance(Vec3::splat(0.1)) < 0.);
        let mut hollow = vertices.clone();
        hollow.extend(vertices.iter().map(|v| SceneVertex {
            position: (Vec3::splat(0.1) + Vec3::from_array(v.position) * 0.2).to_array(),
            ..*v
        }));
        let mut faces = TETRA_FACES.to_vec();
        for face in TETRA_FACES.chunks_exact(3) {
            faces.extend_from_slice(&[face[0] + 4, face[2] + 4, face[1] + 4]);
        }
        let cavity = GraspObject::from_mesh(&hollow, &faces).unwrap();
        assert!(cavity.distance(Vec3::splat(0.05)) < 0.);
        assert!(cavity.distance(Vec3::splat(0.12)) > 0.);
        // A separate inverted shell is invalid even when total signed volume is positive.
        for v in &mut hollow[4..] {
            v.position[0] += 2.;
        }
        assert!(GraspObject::from_mesh(&hollow, &faces).is_err());
        let mut pinch = vertices.clone();
        pinch.extend(vertices.iter().map(|v| SceneVertex {
            position: (-Vec3::from_array(v.position)).to_array(),
            ..*v
        }));
        assert_eq!(
            GraspObject::from_mesh(&pinch, &faces).unwrap_err(),
            "nonmanifold grasp mesh vertex"
        );
    }

    fn handle_object() -> GraspObject {
        let asset = voxy_render::ObjAsset::parse(
            include_str!("/Users/themoretheless/Documents/ChatGPT/Voxy/assets/characters/grasp-handle.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let vertices: Vec<_> = asset
            .mesh
            .vertices()
            .iter()
            .map(|v| SceneVertex {
                position: (Vec3::new(0.360, -0.055, 0.078)
                    + Vec3::from_array(v.position) * Vec3::new(0.625, 0.625, 1.0))
                .to_array(),
                ..*v
            })
            .collect();
        GraspObject::from_mesh(&vertices, asset.mesh.indices()).unwrap()
    }
    #[test]
    fn grasp_fingertip_surface_follows_distal_bones() {
        let asset = voxy_render::ObjAsset::parse(
            include_str!("/Users/themoretheless/Documents/ChatGPT/Voxy/assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let rest = asset.mesh.vertices();
        let mut rig = FemaleRig::new(rest).unwrap();
        rig.bind_hand_surface(rest, asset.mesh.indices()).unwrap();
        for object in [
            handle_object(),
            GraspObject::Sphere {
                center: Vec3::new(0.350, -0.075, 0.078),
                radius: 0.032,
            },
            GraspObject::Cylinder {
                center: Vec3::new(0.350, -0.075, 0.078),
                radius: 0.024,
                half_length: 0.060,
            },
        ] {
            let object = std::sync::Arc::new(object);
            rig.set_grasp_object(Some(object.clone()));
            rig.set_grasp(1.).unwrap();
            let matrices = rig.pose(0.).skin_matrices(&rig.skeleton).unwrap();
            let mut deformed = rest.to_vec();
            rig.deform(&mut deformed, 0.);
            for side in 0..2 {
                for (finger, chain) in FINGER_CHAINS.iter().enumerate() {
                    let sign = if side == 0 { 1. } else { -1. };
                    let tip = Vec3::from_array(chain[3]) * Vec3::new(sign, 1., 1.);
                    let index = rest
                        .iter()
                        .enumerate()
                        .min_by(|(_, a), (_, b)| {
                            Vec3::from_array(a.position)
                                .distance_squared(tip)
                                .total_cmp(&Vec3::from_array(b.position).distance_squared(tip))
                        })
                        .unwrap()
                        .0;
                    let expected = matrices[24 + side * 15 + finger * 3 + 2]
                        .transform_point3(Vec3::from_array(rest[index].position));
                    let error = expected.distance(Vec3::from_array(deformed[index].position));
                    println!("fingertip {side}/{finger} bone-surface error {error:.6} m");
                    let palm_inverse = matrices[if side == 0 { 6 } else { 12 }].inverse();
                    let start = 24 + side * 15 + finger * 3;
                    let skin_gap = rig
                        .finger_weights
                        .iter()
                        .enumerate()
                        .filter_map(|(i, weights)| {
                            let weights = weights.as_ref()?;
                            let influence: f32 = weights
                                .iter()
                                .filter(|(bone, _)| (start..start + 3).contains(bone))
                                .map(|(_, weight)| weight)
                                .sum();
                            if influence < 0.8 {
                                return None;
                            }
                            let mut point = palm_inverse
                                .transform_point3(Vec3::from_array(deformed[i].position));
                            if side == 1 {
                                point.x = -point.x;
                            }
                            Some(object.distance(point))
                        })
                        .fold(f32::INFINITY, f32::min);
                    println!("finger {side}/{finger} actual skin gap {skin_gap:.6} m");
                    assert!(
                        (0. ..0.002).contains(&skin_gap),
                        "finger collider contact disagrees with skin: {side}/{finger}: {skin_gap}"
                    );
                    if side == 0 {
                        let bone_tip = (matrices[6].inverse() * matrices[24 + finger * 3 + 2])
                            .transform_point3(tip);
                        println!(
                            "tip {finger} object distance {:.6} m",
                            object.distance(bone_tip)
                        );
                        let pose = rig.pose(0.);
                        let rotations = std::array::from_fn(|joint| {
                            pose.local()[24 + finger * 3 + joint].rotation
                        });
                        println!(
                            "distal finger {finger} clearance {:.6} m",
                            finger_clearance_from(*chain, rotations, &object, 2)
                        );
                    }
                    assert!(
                        error < 0.00001,
                        "fingertip skin departs from distal bone: {side}/{finger}: {error}"
                    );
                }
            }
        }
    }

    #[test]
    fn grasp_distal_fingers_do_not_cross() {
        fn segment_distance(a: Vec3, b: Vec3, c: Vec3, d: Vec3) -> f32 {
            let u = b - a;
            let v = d - c;
            let w = a - c;
            let aa = u.length_squared();
            let bb = u.dot(v);
            let cc = v.length_squared();
            let dd = u.dot(w);
            let ee = v.dot(w);
            let denominator = aa * cc - bb * bb;
            let mut t = if denominator > 1e-12 {
                ((bb * ee - cc * dd) / denominator).clamp(0., 1.)
            } else {
                0.
            };
            let mut q = (bb * t + ee) / cc;
            if q < 0. {
                q = 0.;
                t = (-dd / aa).clamp(0., 1.);
            } else if q > 1. {
                q = 1.;
                t = ((bb - dd) / aa).clamp(0., 1.);
            }
            (a + u * t).distance(c + v * q)
        }
        for object in [
            handle_object(),
            GraspObject::Sphere {
                center: Vec3::new(0.350, -0.075, 0.078),
                radius: 0.032,
            },
            GraspObject::Cylinder {
                center: Vec3::new(0.350, -0.075, 0.078),
                radius: 0.024,
                half_length: 0.060,
            },
        ] {
            let mut rig = FemaleRig::new(&[]).unwrap();
            rig.set_grasp_object(Some(std::sync::Arc::new(object)));
            let mut minimum = f32::INFINITY;
            for frame in 0..=120 {
                rig.set_grasp(frame as f32 / 120.).unwrap();
                let pose = rig.pose(0.);
                let points: [[Vec3; 4]; 5] = std::array::from_fn(|finger| {
                    let chain = FINGER_CHAINS[finger].map(Vec3::from_array);
                    let mut points = chain;
                    let mut rotation = Quat::IDENTITY;
                    for joint in 0..3 {
                        rotation *= pose.local()[24 + finger * 3 + joint].rotation;
                        points[joint + 1] =
                            points[joint] + rotation * (chain[joint + 1] - chain[joint]);
                    }
                    points
                });
                for first in 0..5 {
                    for second in first + 1..5 {
                        for a in 1..3 {
                            for b in 1..3 {
                                let distance = segment_distance(
                                    points[first][a],
                                    points[first][a + 1],
                                    points[second][b],
                                    points[second][b + 1],
                                );
                                minimum = minimum.min(distance);
                            }
                        }
                    }
                }
            }
            println!("minimum distal finger centerline separation {minimum:.6} m");
            assert!(
                minimum >= 0.0135,
                "distal finger capsules overlap: {minimum}"
            );
        }
    }

    #[test]
    #[ignore = "Offline mesh-based proximal thumb weight optimization"]
    fn thumb_mesh_weight_optimization_diagnostic() {
        let asset = voxy_render::ObjAsset::parse(
            include_str!("/Users/themoretheless/Documents/ChatGPT/Voxy/assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let rest = asset.mesh.vertices();
        let mut rig = FemaleRig::new(rest).unwrap();
        // Rebuild from the analytic baseline to keep baking reproducible.
        rig.finger_weights = rest
            .iter()
            .map(|v| {
                finger_weights_uncorrected(Vec3::from_array(v.position)).map(Vec::into_boxed_slice)
            })
            .collect();
        rig.bind_hand_surface(rest, asset.mesh.indices()).unwrap();
        rig.set_body_motion(false);
        let mut palettes = Vec::new();
        for object in [
            None,
            Some(GraspObject::Cylinder {
                center: Vec3::new(0.350, -0.075, 0.078),
                radius: 0.024,
                half_length: 0.060,
            }),
            Some(GraspObject::Sphere {
                center: Vec3::new(0.350, -0.075, 0.078),
                radius: 0.032,
            }),
            Some(handle_object()),
        ] {
            rig.set_grasp_object(object.map(std::sync::Arc::new));
            for amount in [0.25, 0.50, 0.75, 1.] {
                rig.set_grasp(amount).unwrap();
                let matrices = rig.pose(0.).skin_matrices(&rig.skeleton).unwrap();
                let left = matrices[6].inverse();
                let right = matrices[12].inverse();
                palettes.push(
                    matrices
                        .iter()
                        .enumerate()
                        .map(|(i, m)| {
                            crate::rig_skinning::RigidSkinTransform::from_matrix(if i < 16 {
                                Mat4::IDENTITY
                            } else if i < 31 {
                                left * *m
                            } else {
                                right * *m
                            })
                        })
                        .collect::<Vec<_>>(),
                );
            }
        }
        let mut positions: Vec<Vec<Vec3>> = palettes
            .iter()
            .map(|palette| {
                rest.iter()
                    .enumerate()
                    .map(|(i, v)| {
                        let p = Vec3::from_array(v.position);
                        rig.finger_weights[i]
                            .as_ref()
                            .map_or(p, |w| crate::rig_skinning::deform_point(p, w, palette))
                    })
                    .collect()
            })
            .collect();
        let hinges: Vec<_> = rig.thumb_web_hinges.iter().copied().collect();
        let mut adjacent = vec![Vec::new(); rest.len()];
        for (h, (ids, _)) in hinges.iter().enumerate() {
            for &id in ids {
                adjacent[id].push(h);
            }
        }
        let cost_single = |positions: &[Vec3], indices: &[usize]| -> f32 {
            indices
                .iter()
                .map(|&h| {
                    let (ids, angle) = hinges[h];
                    let p = ids.map(|i| positions[i]);
                    let rp = ids.map(|i| Vec3::from_array(rest[i].position));
                    let ca = (p[1] - p[0]).cross(p[2] - p[0]);
                    let cb = (p[0] - p[1]).cross(p[3] - p[1]);
                    let extra =
                        (ca.normalize().dot(cb.normalize()).clamp(-1., 1.).acos() - angle).max(0.);
                    let area = ca.length() / (rp[1] - rp[0]).cross(rp[2] - rp[0]).length();
                    extra.powi(2)
                        + 40. * (extra - 0.30).max(0.).powi(4)
                        + 100. * (0.65 - area).max(0.).powi(2)
                })
                .sum()
        };
        let cost = |samples: &[Vec<Vec3>], indices: &[usize]| -> f32 {
            samples.iter().map(|p| cost_single(p, indices)).sum()
        };
        let all: Vec<_> = (0..hinges.len()).collect();
        let original = rig.finger_weights.clone();
        println!("WEIGHT OPT before cost {}", cost(&positions, &all));
        for epoch in 0..50 {
            let mut changes = 0;
            for i in 0..rest.len() {
                if adjacent[i].is_empty() {
                    continue;
                }
                let Some(weights) = rig.finger_weights[i].as_mut() else {
                    continue;
                };
                let thumb_root = if rest[i].position[0] >= 0. { 16 } else { 31 };
                let Some(root) = weights.iter().position(|(b, _)| *b == thumb_root) else {
                    continue;
                };
                let palm = weights.iter().position(|(b, _)| *b == 0).unwrap();
                if weights[root].1 < 0.05
                    || original[i]
                        .as_ref()
                        .unwrap()
                        .iter()
                        .filter(|(b, _)| (thumb_root..thumb_root + 3).contains(b))
                        .map(|(_, w)| w)
                        .sum::<f32>()
                        > 0.75
                {
                    continue;
                }
                let current = weights[root].1;
                let total = current + weights[palm].1;
                let baseline = original[i].as_ref().unwrap()[root].1;
                let old = positions.iter().map(|p| p[i]).collect::<Vec<_>>();
                let mut best = (cost(&positions, &adjacent[i]), current, old);
                let increment = 0.02 / (1. + (epoch / 10) as f32);
                for step in [-increment, increment] {
                    let next = (current + step)
                        .clamp((baseline - 0.25).max(0.), (baseline + 0.25).min(total));
                    weights[root].1 = next;
                    weights[palm].1 = total - next;
                    for (points, palette) in positions.iter_mut().zip(&palettes) {
                        points[i] = crate::rig_skinning::deform_point(
                            Vec3::from_array(rest[i].position),
                            weights,
                            palette,
                        );
                    }
                    let candidate = cost(&positions, &adjacent[i]);
                    if candidate < best.0 {
                        best = (candidate, next, positions.iter().map(|p| p[i]).collect());
                    }
                }
                weights[root].1 = best.1;
                weights[palm].1 = total - best.1;
                for (points, p) in positions.iter_mut().zip(best.2) {
                    points[i] = p;
                }
                if (best.1 - current).abs() > 0.0001 {
                    changes += 1;
                }
            }
            println!(
                "WEIGHT OPT epoch {epoch} changes {changes} cost {}",
                cost(&positions, &all)
            );
            if changes == 0 && epoch >= 40 {
                break;
            }
        }
        let mut correction = String::new();
        for (i, weights) in rig.finger_weights.iter().enumerate() {
            let Some(weights) = weights else {
                continue;
            };
            let thumb_root = if rest[i].position[0] >= 0. { 16 } else { 31 };
            let before = original[i].as_ref().unwrap();
            let delta = weights
                .iter()
                .find(|(b, _)| *b == thumb_root)
                .map_or(0., |(_, w)| *w)
                - before
                    .iter()
                    .find(|(b, _)| *b == thumb_root)
                    .map_or(0., |(_, w)| *w);
            if delta.abs() > 0.000001 {
                let p = rest[i].position;
                correction.push_str(&format!(
                    "{:.9},{:.9},{:.9},{:.9}\n",
                    p[0], p[1], p[2], delta
                ));
            }
        }
        std::fs::write(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/hand-thumb-binding-correction.csv"),
            correction,
        )
        .unwrap();
        let positions = &positions[7];
        rig.set_grasp_object(Some(std::sync::Arc::new(GraspObject::Cylinder {
            center: Vec3::new(0.350, -0.075, 0.078),
            radius: 0.024,
            half_length: 0.060,
        })));
        rig.set_grasp(1.).unwrap();
        let mut maximum = 0_f32;
        let mut area = f32::INFINITY;
        for (ids, angle) in &hinges {
            let p = ids.map(|i| positions[i]);
            let rp = ids.map(|i| Vec3::from_array(rest[i].position));
            let ca = (p[1] - p[0]).cross(p[2] - p[0]);
            let cb = (p[0] - p[1]).cross(p[3] - p[1]);
            maximum = maximum.max(ca.normalize().dot(cb.normalize()).clamp(-1., 1.).acos() - angle);
            area = area.min(ca.length() / (rp[1] - rp[0]).cross(rp[2] - rp[0]).length());
        }
        println!("WEIGHT OPT after raw crease {maximum}, min area {area}");
        let mut corrected_worst = 0_f32;
        let mut corrected_area = f32::INFINITY;
        let mut object_gap = f32::INFINITY;
        for frame in 0..=120 {
            rig.set_grasp(frame as f32 / 120.).unwrap();
            let mut corrected = rest.to_vec();
            rig.deform(&mut corrected, 0.);
            for (ids, angle) in &hinges {
                let p = ids.map(|i| Vec3::from_array(corrected[i].position));
                let rp = ids.map(|i| Vec3::from_array(rest[i].position));
                let ca = (p[1] - p[0]).cross(p[2] - p[0]);
                let cb = (p[0] - p[1]).cross(p[3] - p[1]);
                corrected_worst = corrected_worst
                    .max(ca.normalize().dot(cb.normalize()).clamp(-1., 1.).acos() - angle);
                corrected_area =
                    corrected_area.min(ca.length() / (rp[1] - rp[0]).cross(rp[2] - rp[0]).length());
            }
            for (i, v) in corrected.iter().enumerate() {
                let p = Vec3::from_array(rest[i].position);
                if p.x > 0.32 && p.y < 0.03 && p.y > -0.16 {
                    object_gap = object_gap.min(
                        rig.grasp_object
                            .as_ref()
                            .unwrap()
                            .distance(Vec3::from_array(v.position)),
                    );
                }
            }
        }
        println!(
            "WEIGHT OPT corrected full-cycle crease {corrected_worst}, area {corrected_area}, object gap {object_gap}"
        );
        for (shape, object) in [
            None,
            Some(GraspObject::Sphere {
                center: Vec3::new(0.350, -0.075, 0.078),
                radius: 0.032,
            }),
            Some(handle_object()),
        ]
        .into_iter()
        .enumerate()
        {
            rig.set_grasp_object(object.map(std::sync::Arc::new));
            let mut worst = 0_f32;
            for frame in 0..=120 {
                rig.set_grasp(frame as f32 / 120.).unwrap();
                let mut vertices = rest.to_vec();
                rig.deform(&mut vertices, 0.);
                for (ids, angle) in &hinges {
                    let p = ids.map(|i| Vec3::from_array(vertices[i].position));
                    let a = (p[1] - p[0]).cross(p[2] - p[0]).normalize();
                    let b = (p[0] - p[1]).cross(p[3] - p[1]).normalize();
                    worst = worst.max(a.dot(b).clamp(-1., 1.).acos() - angle);
                }
            }
            println!("WEIGHT OPT other shape {shape} crease {worst}");
        }
    }

    #[test]
    #[ignore = "Offline stronger thumb articulation search"]
    fn stronger_thumb_contact_search() {
        let asset = voxy_render::ObjAsset::parse(
            include_str!("/Users/themoretheless/Documents/ChatGPT/Voxy/assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let mut rig = FemaleRig::new(asset.mesh.vertices()).unwrap();
        let object = std::sync::Arc::new(GraspObject::Cylinder {
            center: Vec3::new(0.350, -0.075, 0.078),
            radius: 0.024,
            half_length: 0.060,
        });
        rig.set_grasp_object(Some(object.clone()));
        rig.set_grasp(1.).unwrap();
        let reference = rig.pose(0.).local()[16].rotation;
        let chain = FINGER_CHAINS[0];
        let axes: [Vec3; 3] = std::array::from_fn(|j| {
            (Vec3::from_array(chain[j + 1]) - Vec3::from_array(chain[j]))
                .cross(-Vec3::X)
                .normalize()
        });
        let mut best = None;
        let mut cost = f32::INFINITY;
        let mut stages = [0usize; 4];
        let mut min_root = f32::INFINITY;
        for x in -60..=60 {
            for y in -80..=40 {
                for roll in -6..=6 {
                    let mut q = [
                        Quat::from_rotation_y(y as f32 * 0.01)
                            * Quat::from_rotation_x(x as f32 * 0.01)
                            * reference
                            * Quat::from_axis_angle(
                                (Vec3::from_array(chain[1]) - Vec3::from_array(chain[0]))
                                    .normalize(),
                                roll as f32 * 0.1,
                            ),
                        Quat::from_axis_angle(axes[1], 0.75),
                        Quat::from_axis_angle(axes[2], 0.55),
                    ];
                    let initial = finger_clearance(chain, q, &object);
                    if initial < 0. {
                        continue;
                    }
                    let safe = q[0];
                    let blocked = [-0.01, 0.01].into_iter().find_map(|angle| {
                        let root = Quat::from_rotation_y(angle) * safe;
                        q[0] = root;
                        (finger_clearance(chain, q, &object) < 0.).then_some(root)
                    });
                    let Some(blocked) = blocked else {
                        continue;
                    };
                    let mut low = 0.;
                    let mut high = 1.;
                    for _ in 0..16 {
                        let mid = (low + high) * 0.5;
                        q[0] = safe.slerp(blocked, mid);
                        if finger_clearance(chain, q, &object) >= 0. {
                            low = mid;
                        } else {
                            high = mid;
                        }
                    }
                    q[0] = safe.slerp(blocked, (low - 0.0001).max(0.));
                    stages[0] += 1;
                    let gap = finger_surface_gap(chain, 0, q, &object, &rig.finger_contact_skin[0]);
                    if !(0. ..=0.002).contains(&gap) {
                        continue;
                    }
                    stages[1] += 1;
                    min_root = min_root.min(q[0].angle_between(Quat::IDENTITY));
                    let score = q[0].angle_between(Quat::IDENTITY);
                    if score >= cost {
                        continue;
                    }
                    let mut frames: [usize; 3] =
                        q.map(|v| (v.angle_between(Quat::IDENTITY) / 0.0124).ceil() as usize);
                    frames[1] = frames[1].max(frames[2]);
                    frames[2] = frames[1];
                    if frames.iter().any(|&n| n > 120) {
                        continue;
                    }
                    stages[2] += 1;
                    let mut worst_proxy = (f32::INFINITY, 0usize);
                    let mut worst_skin = (f32::INFINITY, 0usize);
                    for f in 0..=120 {
                        let p = std::array::from_fn(|j| {
                            Quat::IDENTITY.slerp(q[j], (f as f32 / frames[j].max(1) as f32).min(1.))
                        });
                        let proxy = finger_clearance(chain, p, &object);
                        let skin =
                            finger_surface_gap(chain, 0, p, &object, &rig.finger_contact_skin[0]);
                        if proxy < worst_proxy.0 {
                            worst_proxy = (proxy, f);
                        }
                        if skin < worst_skin.0 {
                            worst_skin = (skin, f);
                        }
                    }
                    println!(
                        "STRONG ROUTE {x}/{y}/{roll}: proxy {worst_proxy:?}, skin {worst_skin:?}, q {q:?}"
                    );
                    let timing = (frames[1]..=120).step_by(4).find(|&bend_frames| {
                        (0..=120).all(|f| {
                            let p = std::array::from_fn(|j| {
                                let duration = if j == 0 { frames[0] } else { bend_frames };
                                Quat::IDENTITY
                                    .slerp(q[j], (f as f32 / duration.max(1) as f32).min(1.))
                            });
                            finger_clearance(chain, p, &object) >= 0.
                                && finger_surface_gap(
                                    chain,
                                    0,
                                    p,
                                    &object,
                                    &rig.finger_contact_skin[0],
                                ) >= 0.
                        })
                    });
                    if timing.is_none() {
                        continue;
                    }
                    stages[3] += 1;
                    cost = score;
                    best = Some((x, y, roll, gap, timing, q));
                }
            }
        }
        println!("STRONG THUMB stages {stages:?}, best {best:?}, min root {min_root}");
        assert!(
            best.is_some(),
            "no collision-free thumb contact trajectory found"
        );
        if let Some((_, _, _, _, Some(bend_frames), q)) = best {
            let rest = asset.mesh.vertices();
            rig.bind_hand_surface(rest, asset.mesh.indices()).unwrap();
            rig.set_body_motion(false);
            let mut samples = (**rig.grasp_samples.as_ref().unwrap()).clone();
            let root_frames = (q[0].angle_between(Quat::IDENTITY) / 0.0124).ceil() as usize;
            for (frame, sample) in samples.iter_mut().enumerate() {
                sample[0] = std::array::from_fn(|joint| {
                    let duration = if joint == 0 { root_frames } else { bend_frames };
                    Quat::IDENTITY.slerp(q[joint], (frame as f32 / duration.max(1) as f32).min(1.))
                });
            }
            rig.grasp_samples = Some(std::sync::Arc::new(samples));
            // Keep the cylinder's prepared poses while testing the bending solver.
            // This field only selects surface correction here; contact was checked above.
            rig.grasp_object = None;
            let mut worst = 0_f32;
            let mut minimum_area = f32::INFINITY;
            let mut corrected_gap = f32::INFINITY;
            for frame in 0..=120 {
                rig.set_grasp(frame as f32 / 120.).unwrap();
                let mut posed = rest.to_vec();
                rig.deform(&mut posed, 0.);
                let inverse = [
                    rig.hand_matrix(0., false).inverse(),
                    rig.hand_matrix(0., true).inverse(),
                ];
                for (index, vertex) in rest.iter().enumerate() {
                    if rig.hand_mobility[index] == 0. {
                        continue;
                    }
                    let right = vertex.position[0] < 0.;
                    let p = inverse[usize::from(right)]
                        .transform_point3(Vec3::from_array(posed[index].position));
                    let mirror = Vec3::new(if right { -1. } else { 1. }, 1., 1.);
                    let mut canonical = p * mirror;
                    for _ in 0..3 {
                        let gap = object.distance(canonical);
                        if gap >= 0.000001 {
                            break;
                        }
                        let gradient = Vec3::from_array(std::array::from_fn(|axis| {
                            let mut plus = canonical;
                            let mut minus = canonical;
                            plus[axis] += 0.00001;
                            minus[axis] -= 0.00001;
                            (object.distance(plus) - object.distance(minus)) / 0.00002
                        }));
                        canonical += gradient.normalize_or_zero() * (0.000001 - gap);
                    }
                    posed[index].position = inverse[usize::from(right)]
                        .inverse()
                        .transform_point3(canonical * mirror)
                        .to_array();
                    corrected_gap = corrected_gap.min(object.distance(canonical));
                }
                if frame == 120 {
                    use std::fmt::Write;
                    let mut csv = String::new();
                    for (source, target) in rest.iter().zip(&posed) {
                        let p = source.position;
                        if p[0].abs() > 0.32 && (-0.16..0.03).contains(&p[1]) {
                            writeln!(
                                csv,
                                "{},{},{},{},{},{}",
                                p[0],
                                p[1],
                                p[2],
                                target.position[0],
                                target.position[1],
                                target.position[2]
                            )
                            .unwrap();
                        }
                    }
                    std::fs::write(
                        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                            .join("../../target/hand-candidate-closed.csv"),
                        csv,
                    )
                    .unwrap();
                }
                for &(ids, rest_angle) in &rig.thumb_web_hinges {
                    let p = ids.map(|i| Vec3::from_array(posed[i].position));
                    let rp = ids.map(|i| Vec3::from_array(rest[i].position));
                    let a = (p[1] - p[0]).cross(p[2] - p[0]);
                    let b = (p[0] - p[1]).cross(p[3] - p[1]);
                    minimum_area = minimum_area
                        .min(a.length() / (rp[1] - rp[0]).cross(rp[2] - rp[0]).length());
                    worst = worst
                        .max(a.normalize().dot(b.normalize()).clamp(-1., 1.).acos() - rest_angle);
                }
            }
            println!(
                "INWARD THUMB full-cycle extra crease {worst}, min area {minimum_area}, corrected gap {corrected_gap}"
            );
            assert!(
                corrected_gap >= 0.,
                "corrected skin penetrates the cylinder"
            );
            assert!(minimum_area > 0.55, "corrected thumb web collapses");
            assert!(
                worst < 0.30,
                "candidate still forms a sharp thumb web crease: {worst}"
            );
        }
    }

    #[test]
    #[ignore = "Offline thumb contact envelope diagnostic"]
    fn thumb_contact_envelope_diagnostic() {
        let asset = voxy_render::ObjAsset::parse(
            include_str!("/Users/themoretheless/Documents/ChatGPT/Voxy/assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let mut rig = FemaleRig::new(asset.mesh.vertices()).unwrap();
        let object = std::sync::Arc::new(GraspObject::Cylinder {
            center: Vec3::new(0.350, -0.075, 0.078),
            radius: 0.024,
            half_length: 0.060,
        });
        rig.set_grasp_object(Some(object.clone()));
        rig.set_grasp(1.).unwrap();
        let pose = rig.pose(0.);
        let closed: [Quat; 3] = std::array::from_fn(|joint| pose.local()[24 + joint].rotation);
        let earliest = (0..=120).find(|&frame| {
            rig.set_grasp(frame as f32 / 120.).unwrap();
            let frame_pose = rig.pose(0.);
            (0..3).all(|j| frame_pose.local()[24 + j].rotation.angle_between(closed[j]) < 0.001)
        });
        println!("THUMB earliest closed {earliest:?}");
        let axes: [Vec3; 3] = std::array::from_fn(|joint| {
            (Vec3::from_array(FINGER_CHAINS[0][joint + 1])
                - Vec3::from_array(FINGER_CHAINS[0][joint]))
            .cross(Vec3::X)
            .normalize()
        });
        let mut best = None;
        let mut stages = [0usize; 3];
        let mut minimum_skin = f32::INFINITY;
        let mut best_cost = f32::INFINITY;
        for middle in 2..=8 {
            for distal in 1..=6 {
                for x in -25..=25 {
                    for y in -25..=25 {
                        let mut candidate = [
                            Quat::from_rotation_y(y as f32 * 0.01)
                                * Quat::from_rotation_x(x as f32 * 0.01)
                                * closed[0],
                            closed[1] * Quat::from_axis_angle(axes[1], middle as f32 * 0.05),
                            closed[2] * Quat::from_axis_angle(axes[2], distal as f32 * 0.05),
                        ];
                        let cost = (middle as f32 * 0.05 - 0.30).powi(2)
                            + (distal as f32 * 0.05 - 0.20).powi(2)
                            + 0.1 * ((x as f32 * 0.01).powi(2) + (y as f32 * 0.01).powi(2));
                        if cost >= best_cost {
                            continue;
                        }
                        let mut proxy = finger_clearance(FINGER_CHAINS[0], candidate, &object);
                        if proxy < 0. {
                            let blocked = candidate[0];
                            let safe = Quat::from_rotation_y((y - 1) as f32 * 0.01)
                                * Quat::from_rotation_x(x as f32 * 0.01)
                                * closed[0];
                            candidate[0] = safe;
                            if finger_clearance(FINGER_CHAINS[0], candidate, &object) < 0. {
                                continue;
                            }
                            let mut low = 0.;
                            let mut high = 1.;
                            for _ in 0..16 {
                                let mid = (low + high) * 0.5;
                                candidate[0] = safe.slerp(blocked, mid);
                                if finger_clearance(FINGER_CHAINS[0], candidate, &object) >= 0. {
                                    low = mid;
                                } else {
                                    high = mid;
                                }
                            }
                            candidate[0] = safe.slerp(blocked, low);
                            proxy = finger_clearance(FINGER_CHAINS[0], candidate, &object);
                        }
                        if !(0. ..=0.00005).contains(&proxy) {
                            continue;
                        }
                        stages[0] += 1;
                        let gap = finger_surface_gap(
                            FINGER_CHAINS[0],
                            0,
                            candidate,
                            &object,
                            &rig.finger_contact_skin[0],
                        );
                        minimum_skin = minimum_skin.min(gap);
                        if !(0. ..=0.002).contains(&gap) {
                            continue;
                        }
                        stages[1] += 1;
                        if !(1..=32).all(|step| {
                            let intermediate = std::array::from_fn(|j| {
                                let amount = step as f32 / 32.;
                                let phase = if j == 0 {
                                    (amount * 2. - 1.).max(0.)
                                } else {
                                    (amount * 2.).min(1.)
                                };
                                closed[j].slerp(candidate[j], phase)
                            });
                            finger_clearance(FINGER_CHAINS[0], intermediate, &object) >= 0.
                        }) {
                            continue;
                        }
                        let timed = (0..=100).find_map(|start| {
                            let base = rig.grasp_samples.as_ref().unwrap()[start][0];
                            let bend_frames = ((1..3)
                                .map(|j| base[j].angle_between(candidate[j]))
                                .fold(0_f32, f32::max)
                                / 0.012)
                                .ceil() as usize;
                            let root_frames =
                                (base[0].angle_between(candidate[0]) / 0.012).ceil() as usize;
                            (0..=30).step_by(5).find_map(|delay| {
                                let length = bend_frames.max(delay + root_frames);
                                if start + length > 120 {
                                    return None;
                                }
                                let valid = (1..=length).all(|frame| {
                                    let q = std::array::from_fn(|j| {
                                        let phase = if j == 0 {
                                            (frame.saturating_sub(delay) as f32
                                                / root_frames.max(1) as f32)
                                                .min(1.)
                                        } else {
                                            (frame as f32 / bend_frames.max(1) as f32).min(1.)
                                        };
                                        base[j].slerp(candidate[j], phase)
                                    });
                                    finger_clearance(FINGER_CHAINS[0], q, &object) >= 0.
                                        && finger_surface_gap(
                                            FINGER_CHAINS[0],
                                            0,
                                            q,
                                            &object,
                                            &rig.finger_contact_skin[0],
                                        ) >= 0.
                                });
                                valid.then_some((start, bend_frames, root_frames, delay))
                            })
                        });
                        println!("THUMB timed candidate {x}/{y}/{middle}/{distal}: {timed:?}");
                        stages[2] += 1;
                        best_cost = cost;
                        best = Some((x, y, middle, distal, proxy, gap));
                    }
                }
            }
        }
        println!("COUPLED THUMB best {best:?}, stages {stages:?}, minimum skin {minimum_skin}");
        for joint in 0..3 {
            let axis = (Vec3::from_array(FINGER_CHAINS[0][joint + 1])
                - Vec3::from_array(FINGER_CHAINS[0][joint]))
            .cross(Vec3::X)
            .normalize();
            for step in 0..=6 {
                let mut candidate = closed;
                candidate[joint] *= Quat::from_axis_angle(axis, step as f32 * 0.05);
                println!(
                    "thumb joint {joint} extra {:.2}: proxy {:.6}, actual skin {:.6}",
                    step as f32 * 0.05,
                    finger_clearance(FINGER_CHAINS[0], candidate, &object),
                    finger_surface_gap(
                        FINGER_CHAINS[0],
                        0,
                        candidate,
                        &object,
                        &rig.finger_contact_skin[0]
                    )
                );
            }
        }
    }

    #[test]
    fn object_contact_limits_each_finger_without_penetration() {
        let asset = voxy_render::ObjAsset::parse(
            include_str!("/Users/themoretheless/Documents/ChatGPT/Voxy/assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let mesh = handle_object();
        for object in [
            mesh,
            GraspObject::Sphere {
                center: Vec3::new(0.350, -0.075, 0.078),
                radius: 0.032,
            },
            GraspObject::Cylinder {
                center: Vec3::new(0.350, -0.075, 0.078),
                radius: 0.024,
                half_length: 0.060,
            },
        ] {
            let object = std::sync::Arc::new(object);
            let mut rig = FemaleRig::new(asset.mesh.vertices()).unwrap();
            rig.set_grasp_object(Some(object.clone()));
            for (finger, chain) in FINGER_CHAINS.into_iter().enumerate() {
                rig.set_grasp(1.).unwrap();
                let pose = rig.pose(0.);
                let fitted = std::array::from_fn(|i| pose.local()[24 + finger * 3 + i].rotation);
                let clearance = finger_clearance(chain, fitted, &object);
                let mut previous = [Quat::IDENTITY; 3];
                let mut max_jump = 0_f32;
                for frame in 1..=120 {
                    rig.set_grasp(frame as f32 / 120.).unwrap();
                    let pose = rig.pose(0.);
                    let current =
                        std::array::from_fn(|i| pose.local()[24 + finger * 3 + i].rotation);
                    assert!(finger_clearance(chain, current, &object) >= -1.1e-6);
                    for joint in 0..3 {
                        max_jump = max_jump.max(previous[joint].angle_between(current[joint]));
                    }
                    previous = current;
                }
                if finger == 0 && matches!(object.as_ref(), GraspObject::Cylinder { .. }) {
                    assert!(
                        (1..3).all(|j| {
                            let q=fitted[j];let axis=FingerJointAxes::for_segment(chain,j,1.).flexion;
                            Vec3::new(q.x,q.y,q.z).dot(axis)>0.
                                && q.angle_between(Quat::IDENTITY)<=std::f32::consts::FRAC_PI_2
                        }),
                        "cylinder thumb phalanges must flex toward the palm within the authored range"
                    );
                }
                if finger > 0 {
                    let mut rotation = Quat::IDENTITY;
                    let mut point = Vec3::from_array(chain[0]);
                    for joint in 0..3 {
                        rotation *= fitted[joint];
                        let direction =
                            Vec3::from_array(chain[joint + 1]) - Vec3::from_array(chain[joint]);
                        point += rotation * direction;
                        let mut candidate = fitted;
                        candidate[joint] *=
                            Quat::from_axis_angle(direction.cross(-Vec3::X).normalize(), 0.1);
                        println!(
                            "CONTACT DIAG {finger}/{joint}: endpoint={point:?}, distance={:.6}, extra-flex-clear={:.6}",
                            object.distance(point),
                            finger_clearance_from(chain, candidate, &object, joint)
                        );
                    }
                }
                println!("contact angular step {finger}: {max_jump:.4}");
                assert!(
                    max_jump < 0.013,
                    "contact pose snaps at finger {finger}: {max_jump}"
                );
                println!(
                    "finger {finger}: clear {clearance:.6} angles {:?}",
                    fitted.map(|q| q.to_axis_angle().1)
                );
                assert!(
                    clearance >= -1.1e-6,
                    "contact penetrates {finger}: {clearance}"
                );
                assert!(
                    clearance
                        < if finger == 0 {
                            0.00001
                        } else if finger == 4 {
                            0.0001
                        } else {
                            0.005
                        },
                    "finger {finger} misses the grasp object: {clearance}"
                );
                assert!(fitted.iter().all(|q| q.is_finite() && q.is_normalized()));
                if finger > 0 {
                    let direction = Vec3::from_array(chain[1]) - Vec3::from_array(chain[0]);
                    let flex_axis = direction.cross(-Vec3::X).normalize();
                    let q = fitted[0];
                    // Lateral splay must not disguise an extended knuckle.
                    // These fixtures require at least ~8 degrees of MCP flexion.
                    assert!(
                        Vec3::new(q.x, q.y, q.z).dot(flex_axis) > 0.07,
                        "grasp leaves knuckle extended at finger {finger}"
                    );
                }
            }
        }
    }
    #[test]
    fn grasp_skin_surface_stays_outside_object() {
        let asset = voxy_render::ObjAsset::parse(
            include_str!("/Users/themoretheless/Documents/ChatGPT/Voxy/assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let rest = asset.mesh.vertices();
        for object in [
            handle_object(),
            GraspObject::Sphere {
                center: Vec3::new(0.350, -0.075, 0.078),
                radius: 0.032,
            },
            GraspObject::Cylinder {
                center: Vec3::new(0.350, -0.075, 0.078),
                radius: 0.024,
                half_length: 0.060,
            },
        ] {
            let object = std::sync::Arc::new(object);
            let mut rig = FemaleRig::new(rest).unwrap();
            rig.bind_hand_surface(rest, asset.mesh.indices()).unwrap();
            rig.set_grasp_object(Some(object.clone()));
            let mut minimum_area = f32::INFINITY;
            let mut maximum_edge = 0_f32;
            let mut minimum = f32::INFINITY;
            let mut worst = Vec3::ZERO;
            for cycle in [false, true] {
                rig.set_grasp_cycle(cycle);
                for frame in 0..=120 {
                    if !cycle {
                        rig.set_grasp(frame as f32 / 120.).unwrap();
                    }
                    let time = frame as f32 * 0.05;
                    let mut posed = rest.to_vec();
                    rig.deform(&mut posed, time);
                    for face in asset.mesh.indices().chunks_exact(3) {
                        let a: [Vec3; 3] = std::array::from_fn(|i| {
                            Vec3::from_array(rest[face[i] as usize].position)
                        });
                        let b: [Vec3; 3] = std::array::from_fn(|i| {
                            Vec3::from_array(posed[face[i] as usize].position)
                        });
                        let area = (a[1] - a[0]).cross(a[2] - a[0]).length();
                        if area > 1e-10 {
                            minimum_area =
                                minimum_area.min((b[1] - b[0]).cross(b[2] - b[0]).length() / area);
                        }
                        for i in 0..3 {
                            let j = (i + 1) % 3;
                            let length = a[i].distance(a[j]);
                            if length > 1e-6 {
                                maximum_edge = maximum_edge.max(b[i].distance(b[j]) / length);
                            }
                        }
                    }
                    let inverse = [
                        rig.hand_matrix(time, false).inverse(),
                        rig.hand_matrix(time, true).inverse(),
                    ];
                    for (index, v) in rest.iter().enumerate() {
                        if rig.hand_mobility[index] == 0. {
                            continue;
                        }
                        let right = v.position[0] < 0.;
                        let p = inverse[usize::from(right)]
                            .transform_point3(Vec3::from_array(posed[index].position));
                        let p = p * Vec3::new(if right { -1. } else { 1. }, 1., 1.);
                        let distance = object.distance(p);
                        if distance < minimum {
                            minimum = distance;
                            worst = Vec3::from_array(v.position);
                        }
                    }
                }
            }
            println!("contact skin area {minimum_area:.4}, edge {maximum_edge:.4}");
            assert!(
                minimum_area > 0.55,
                "contact skin collapses: {minimum_area}"
            );
            assert!(
                maximum_edge < 1.65,
                "contact skin stretches: {maximum_edge}"
            );
            println!("skin contact minimum {minimum:.6} at {worst:?}");
            assert!(
                minimum >= 0.,
                "render skin penetrates object by {} mm",
                -minimum * 1000.
            );
        }
    }
    #[test]
    fn prepared_grasp_interpolation_preserves_contact() {
        for object in [
            GraspObject::Sphere {
                center: Vec3::new(0.350, -0.075, 0.078),
                radius: 0.032,
            },
            GraspObject::Cylinder {
                center: Vec3::new(0.350, -0.075, 0.078),
                radius: 0.024,
                half_length: 0.060,
            },
        ] {
            let object = std::sync::Arc::new(object);
            let mut rig = FemaleRig::new(&[]).unwrap();
            rig.set_grasp_object(Some(object.clone()));
            let mut minimum = f32::INFINITY;
            for frame in 0..=240 {
                rig.set_grasp(frame as f32 / 240.).unwrap();
                let pose = rig.pose(0.);
                for (finger, chain) in FINGER_CHAINS.into_iter().enumerate() {
                    let rotations =
                        std::array::from_fn(|i| pose.local()[24 + finger * 3 + i].rotation);
                    minimum = minimum.min(finger_clearance(chain, rotations, &object));
                    for i in 0..3 {
                        let left = rotations[i];
                        let right = pose.local()[39 + finger * 3 + i].rotation;
                        assert!(
                            right.abs_diff_eq(
                                Quat::from_xyzw(left.x, -left.y, -left.z, left.w),
                                1e-6
                            )
                        );
                    }
                }
            }
            println!("prepared contact min {minimum:.8}");
            assert!(minimum > -1e-5, "interpolation crosses object: {minimum}");
        }
    }
    #[test]
    fn grasp_closure_preserves_surface_and_moves_fingertips() {
        let asset = voxy_render::ObjAsset::parse(
            include_str!("/Users/themoretheless/Documents/ChatGPT/Voxy/assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let rest = asset.mesh.vertices();
        let mut rig = FemaleRig::new(rest).unwrap();
        rig.bind_hand_surface(rest, asset.mesh.indices()).unwrap();
        for invalid in [f32::NAN, -0.1, 1.1] {
            assert!(rig.set_grasp(invalid).is_err());
        }
        let mut area = f32::INFINITY;
        let mut edge = 0_f32;
        let mut worst = (Vec3::ZERO, Vec3::ZERO);
        for frame in 0..=20 {
            rig.set_grasp(frame as f32 / 20.).unwrap();
            let mut posed = rest.to_vec();
            rig.deform(&mut posed, 0.);
            for face in asset.mesh.indices().chunks_exact(3) {
                let a: [Vec3; 3] =
                    std::array::from_fn(|i| Vec3::from_array(rest[face[i] as usize].position));
                let b: [Vec3; 3] =
                    std::array::from_fn(|i| Vec3::from_array(posed[face[i] as usize].position));
                let original_area = (a[1] - a[0]).cross(a[2] - a[0]).length();
                if original_area > 1e-10 {
                    area = area.min((b[1] - b[0]).cross(b[2] - b[0]).length() / original_area);
                }
                for i in 0..3 {
                    let j = (i + 1) % 3;
                    let length = a[i].distance(a[j]);
                    if length > 1e-6 {
                        let ratio = b[i].distance(b[j]) / length;
                        if ratio > edge {
                            edge = ratio;
                            worst = (a[i], a[j]);
                        }
                    }
                }
            }
        }
        rig.set_grasp(0.).unwrap();
        let mut open = rest.to_vec();
        rig.deform(&mut open, 0.);
        rig.set_grasp(1.).unwrap();
        let mut closed = rest.to_vec();
        rig.deform(&mut closed, 0.);
        for sign in [1., -1.] {
            for chain in FINGER_CHAINS {
                let tip = Vec3::from_array(chain[3]) * Vec3::new(sign, 1., 1.);
                let index = rest
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| {
                        Vec3::from_array(a.position)
                            .distance_squared(tip)
                            .total_cmp(&Vec3::from_array(b.position).distance_squared(tip))
                    })
                    .unwrap()
                    .0;
                let movement = Vec3::from_array(open[index].position)
                    .distance(Vec3::from_array(closed[index].position));
                assert!(
                    movement > 0.005,
                    "finger tip did not close: {tip:?}, {movement}"
                );
            }
        }
        for (index, vertex) in rest.iter().enumerate() {
            if vertex.position[0].abs() < 0.25 {
                assert_eq!(
                    open[index].position, closed[index].position,
                    "hand closure moved the torso at {index}"
                );
            }
        }
        println!("GRASP SURFACE: min area {area:.4}, max edge {edge:.4}, worst {worst:?}");
        assert!(area > 0.55, "grasp collapses surface: {area}");
        assert!(edge < 1.65, "grasp stretches surface: {edge}");
    }
    #[test]
    fn gesture_holds_and_loop_have_no_visible_velocity_jump() {
        for time in [0., 0.5, 2.5, 3., 5.5, 6.] {
            let before = gesture_envelope(time - 0.001);
            let after = gesture_envelope(time + 0.001);
            assert!((after - before).abs() < 1e-5, "boundary {time}");
        }
        assert_eq!(gesture_envelope(0.25), 0.);
        assert_eq!(gesture_envelope(2.75), 1.);
        assert_eq!(gesture_envelope(5.75), 0.);
    }
    #[test]
    fn arms_do_not_pull_torso_or_opposite_limbs() {
        for p in [Vec3::new(0.10, 0.1, 0.1), Vec3::new(0., 0.7, 0.)] {
            assert!(anatomical_weights(p).iter().all(|&(i, w)| w == 0. || i < 4));
        }
        for sign in [-1., 1.] {
            let weights = anatomical_weights(Vec3::new(sign * 0.36, 0., 0.));
            assert!(weights.iter().all(|&(i, w)| w == 0.
                || if sign > 0. {
                    (4..7).contains(&i)
                } else {
                    (10..13).contains(&i)
                }));
            for y in [-0.25, -0.44, -0.72, -0.79] {
                let weights = anatomical_weights(Vec3::new(sign * 0.10, y, 0.));
                assert!(weights.iter().all(|&(i, w)| w == 0.
                    || if sign > 0. {
                        (7..10).contains(&i)
                    } else {
                        (13..16).contains(&i)
                    }));
                assert!((weights.iter().map(|w| w.1).sum::<f32>() - 1.).abs() < 1e-6);
            }
        }
    }
    #[test]
    fn proximal_thumb_follows_thumb_bones_in_both_hands() {
        for side in 0..2 {
            let sign = if side == 0 { 1. } else { -1. };
            let joint = FINGER_CHAINS[0][1];
            let weights = finger_weights(Vec3::new(sign * joint[0], joint[1], joint[2]))
                .expect("thumb joint must have finger skin weights");
            let start = 24 + side * 15;
            let thumb: f32 = weights
                .iter()
                .filter(|(bone, _)| (start..start + 3).contains(bone))
                .map(|(_, weight)| weight)
                .sum();
            assert!(
                thumb > 0.75,
                "proximal thumb remains attached to palm: {thumb}"
            );
        }
    }
    #[test]
    #[ignore = "Offline search for feasible cylinder grasp joint targets"]
    fn cylinder_grasp_joint_search() {
        let object = GraspObject::Cylinder {
            center: Vec3::new(0.350, -0.075, 0.078),
            radius: 0.024,
            half_length: 0.060,
        };
        for finger in 1..5 {
            let chain = FINGER_CHAINS[finger];
            let axes: [Vec3; 3] = std::array::from_fn(|joint| {
                (Vec3::from_array(chain[joint + 1]) - Vec3::from_array(chain[joint]))
                    .cross(-Vec3::X)
                    .normalize()
            });
            let mut best = (f32::INFINITY, [0_f32; 3], Vec3::ZERO);
            for root in 4..=30 {
                for middle in 10..=44 {
                    for distal in 8..=26 {
                        let angles = [
                            root as f32 * 0.025,
                            middle as f32 * 0.025,
                            distal as f32 * 0.025,
                        ];
                        let pose = std::array::from_fn(|joint| {
                            Quat::from_axis_angle(axes[joint], angles[joint])
                        });
                        let clearance = finger_clearance(chain, pose, &object);
                        if clearance < 0. || clearance > 0.0005 {
                            continue;
                        }
                        let mut point = Vec3::from_array(chain[0]);
                        let mut rotation = Quat::IDENTITY;
                        for joint in 0..3 {
                            rotation *= pose[joint];
                            point += rotation
                                * (Vec3::from_array(chain[joint + 1])
                                    - Vec3::from_array(chain[joint]));
                        }
                        if object.distance(point) > 0.0085 {
                            continue;
                        }
                        let cost = (angles[0] - 0.50).powi(2)
                            + (angles[1] - 1.10).powi(2)
                            + (angles[2] - 0.65).powi(2);
                        if cost < best.0 {
                            best = (cost, angles, point);
                        }
                    }
                }
            }
            println!("FEASIBLE cylinder finger {finger}: {best:?}");
            assert!(
                best.0.is_finite(),
                "no curled contact pose for finger {finger}"
            );
        }
    }
    #[test]
    fn thumb_web_does_not_form_a_sharp_new_crease() {
        measure_thumb_web_crease(3);
    }
    #[test]
    #[ignore = "Offline skinning-versus-surface-correction diagnostic"]
    fn thumb_web_uncorrected_crease_diagnostic() {
        measure_thumb_web_crease(0);
    }
    #[test]
    #[ignore = "Offline edge-only skin correction diagnostic"]
    fn thumb_web_edge_only_diagnostic() {
        measure_thumb_web_crease(1);
    }
    #[test]
    #[ignore = "Offline area-only skin correction diagnostic"]
    fn thumb_web_area_only_diagnostic() {
        measure_thumb_web_crease(2);
    }
    fn measure_thumb_web_crease(correction_mode: u8) {
        let asset = voxy_render::ObjAsset::parse(
            include_str!("/Users/themoretheless/Documents/ChatGPT/Voxy/assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let rest = asset.mesh.vertices();
        let mut rig = FemaleRig::new(rest).unwrap();
        rig.bind_hand_surface(rest, asset.mesh.indices()).unwrap();
        if correction_mode & 1 == 0 {
            rig.hand_edges.clear();
        }
        if correction_mode & 2 == 0 {
            rig.hand_faces.clear();
        }
        rig.set_body_motion(false);
        assert!(!rig.thumb_web_hinges.is_empty());
        let mut worst = (0_f32, Vec3::ZERO, Vec3::ZERO);
        let mut worst_source = ([0usize; 4], 0usize, 0usize);
        for (shape, object) in [
            None,
            Some(GraspObject::Cylinder {
                center: Vec3::new(0.350, -0.075, 0.078),
                radius: 0.024,
                half_length: 0.060,
            }),
            Some(GraspObject::Sphere {
                center: Vec3::new(0.350, -0.075, 0.078),
                radius: 0.032,
            }),
            Some(handle_object()),
        ]
        .into_iter()
        .enumerate()
        {
            rig.set_grasp_object(object.map(std::sync::Arc::new));
            let mut shape_worst = 0_f32;
            let mut minimum_area = f32::INFINITY;
            for frame in 0..=120 {
                rig.set_grasp(frame as f32 / 120.).unwrap();
                let mut posed = rest.to_vec();
                rig.deform(&mut posed, 0.);
                for &(ids, rest_angle) in &rig.thumb_web_hinges {
                    let p = ids.map(|i| Vec3::from_array(posed[i].position));
                    let rp = ids.map(|i| Vec3::from_array(rest[i].position));
                    let ratio = (p[1] - p[0]).cross(p[2] - p[0]).length()
                        / (rp[1] - rp[0]).cross(rp[2] - rp[0]).length();
                    minimum_area = minimum_area.min(ratio);
                    let a = (p[1] - p[0]).cross(p[2] - p[0]).normalize();
                    let b = (p[0] - p[1]).cross(p[3] - p[1]).normalize();
                    let extra = a.dot(b).clamp(-1., 1.).acos() - rest_angle;
                    assert!(extra.is_finite(), "thumb web triangle becomes degenerate");
                    shape_worst = shape_worst.max(extra);
                    if extra > worst.0 {
                        worst_source = (ids, shape, frame);
                        worst = (
                            extra,
                            (p[0] + p[1]) * 0.5,
                            (Vec3::from_array(rest[ids[0]].position)
                                + Vec3::from_array(rest[ids[1]].position))
                                * 0.5,
                        );
                    }
                }
            }
            println!("thumb web shape {shape}: {shape_worst}, min area {minimum_area}");
        }
        println!("thumb web extra crease: {worst:?}, source {worst_source:?}");
        for id in worst_source.0 {
            println!(
                "crease vertex {id} rest {:?}, weights {:?}",
                rest[id].position, rig.finger_weights[id]
            );
        }
        assert!(
            worst.0 < 0.30,
            "thumb web forms a sharp new crease: {worst:?}"
        );
    }
    #[test]
    fn isolated_grasp_keeps_wrists_fixed_while_fingers_close() {
        let mut rig = FemaleRig::new(&[]).unwrap();
        rig.set_grasp_object(Some(std::sync::Arc::new(GraspObject::Cylinder {
            center: Vec3::new(0.350, -0.075, 0.078),
            radius: 0.024,
            half_length: 0.060,
        })));
        rig.set_body_motion(false);
        rig.set_grasp_cycle(true);
        for right in [false, true] {
            let wrist = rig.hand_matrix(0., right);
            for frame in 0..=120 {
                assert!(
                    rig.hand_matrix(frame as f32 * 0.05, right)
                        .abs_diff_eq(wrist, 1e-6)
                );
            }
        }
        let open = rig.pose(0.).skin_matrices(&rig.skeleton).unwrap();
        let closed = rig.pose(3.).skin_matrices(&rig.skeleton).unwrap();
        let tip = Vec3::from_array(FINGER_CHAINS[1][3]);
        assert!(
            open[29]
                .transform_point3(tip)
                .distance(closed[29].transform_point3(tip))
                > 0.02
        );
        rig.set_body_motion(true);
        assert!(
            !rig.hand_matrix(0., false)
                .abs_diff_eq(rig.hand_matrix(3., false), 1e-6)
        );
    }
    #[test]
    fn anatomical_joint_axes_are_orthonormal_palmar_and_mirrored() {
        let mirror_point = |p: Vec3| Vec3::new(-p.x, p.y, p.z);
        let mirror_axis = |p: Vec3| Vec3::new(p.x, -p.y, -p.z);
        for chain in FINGER_CHAINS {
            for segment in 0..3 {
                let left = FingerJointAxes::for_segment(chain, segment, 1.);
                let right = FingerJointAxes::for_segment(chain, segment, -1.);
                let d = (Vec3::from_array(chain[segment + 1]) - Vec3::from_array(chain[segment]))
                    .normalize();
                for axes in [left, right] {
                    assert!((axes.flexion.length() - 1.).abs() < 1e-6);
                    assert!((axes.spread.length() - 1.).abs() < 1e-6);
                    assert!((axes.twist.length() - 1.).abs() < 1e-6);
                    assert!(axes.flexion.dot(axes.spread).abs() < 1e-6);
                    assert!(axes.flexion.dot(axes.twist).abs() < 1e-6);
                    assert!(axes.spread.dot(axes.twist).abs() < 1e-6);
                    assert!(axes.flexion.cross(axes.spread).distance(axes.twist) < 1e-6);
                }
                for (a, b) in [
                    (left.flexion, right.flexion),
                    (left.spread, right.spread),
                    (left.twist, right.twist),
                ] {
                    assert!(mirror_axis(a).distance(b) < 1e-6);
                }
                let ql = Quat::from_axis_angle(left.flexion, 0.01);
                let qr = Quat::from_axis_angle(right.flexion, 0.01);
                assert!((ql * d - d).dot(Vec3::NEG_X) > 0.);
                assert!((qr * mirror_point(d) - mirror_point(d)).dot(Vec3::X) > 0.);
                assert!(mirror_point(ql * d).distance(qr * mirror_point(d)) < 1e-6);
                let left_control = left.rotation(0.5, 0.15, -0.2);
                let right_control = right.rotation(0.5, 0.15, -0.2);
                for point in [Vec3::X, Vec3::Y, Vec3::Z, d] {
                    assert!(
                        mirror_point(left_control * point)
                            .distance(right_control * mirror_point(point))
                            < 1e-6
                    );
                }
            }
        }
    }

    #[test]
    fn every_finger_joint_articulates_actual_mesh_without_moving_other_hand() {
        let asset = voxy_render::ObjAsset::parse(
            include_str!("/Users/themoretheless/Documents/ChatGPT/Voxy/assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let original = asset.mesh.vertices();
        let mut rig = FemaleRig::new(original).unwrap();
        for weights in rig.finger_weights.iter().flatten() {
            assert!(
                weights
                    .iter()
                    .all(|(_, weight)| weight.is_finite() && *weight >= -1e-6)
            );
            assert!((weights.iter().map(|(_, weight)| weight).sum::<f32>() - 1.).abs() < 1e-6);
        }
        for side in 0..2 {
            let sign = if side == 0 { 1. } else { -1. };
            for (finger, chain) in FINGER_CHAINS.iter().enumerate() {
                for segment in 0..3 {
                    let joint = 24 + side * 15 + finger * 3 + segment;
                    let mirror = |p: [f32; 3]| Vec3::new(sign * p[0], p[1], p[2]);
                    let axis = FingerJointAxes::for_segment(*chain, segment, sign).flexion;
                    let mut tracks = vec![JointTrack::default(); rig.skeleton.joints().len()];
                    tracks[joint].rotations = vec![QuatKey {
                        time: 0.,
                        value: Quat::from_axis_angle(axis, 0.1),
                    }];
                    rig.clip = AnimationClip::new(
                        "isolated finger",
                        1.,
                        Playback::Clamp,
                        tracks,
                        &rig.skeleton,
                    )
                    .unwrap();
                    let mut posed = original.to_vec();
                    rig.deform(&mut posed, 0.);
                    let mut movement = 0_f32;
                    for (rest, now) in original.iter().zip(&posed) {
                        let p = Vec3::from_array(rest.position);
                        let delta = p.distance(Vec3::from_array(now.position));
                        if p.distance(mirror(chain[3])) < 0.025 {
                            movement = movement.max(delta);
                        }
                        if p.x * sign < 0.32 || p.y > 0.03 || p.y < -0.16 {
                            assert!(
                                delta < 1e-6,
                                "joint {joint} moved unrelated vertex {p:?}: {delta}"
                            );
                        }
                    }
                    assert!(
                        movement > 0.0005,
                        "joint {joint} failed to articulate its fingertip: {movement}"
                    );
                }
            }
        }
    }
    #[test]
    fn actual_model_hips_knees_and_ankles_move_independently() {
        let asset = voxy_render::ObjAsset::parse(
            include_str!("/Users/themoretheless/Documents/ChatGPT/Voxy/assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let original = asset.mesh.vertices();
        for (joint, sign, upper_y, minimum_motion) in [
            (7, 1., -0.25, 0.05),
            (8, 1., -0.60, 0.03),
            (9, 1., -0.77, 0.01),
            (13, -1., -0.25, 0.05),
            (14, -1., -0.60, 0.03),
            (15, -1., -0.77, 0.01),
        ] {
            let mut rig = FemaleRig::new(original).unwrap();
            let mut tracks = vec![JointTrack::default(); rig.skeleton.joints().len()];
            tracks[joint].rotations = vec![QuatKey {
                time: 0.,
                value: Quat::from_rotation_x(0.4),
            }];
            rig.clip = AnimationClip::new(
                "isolated leg joint",
                1.,
                Playback::Clamp,
                tracks,
                &rig.skeleton,
            )
            .unwrap();
            let mut posed = original.to_vec();
            rig.deform(&mut posed, 0.);
            let mut movement = 0_f32;
            for (rest, now) in original.iter().zip(&posed) {
                let p = Vec3::from_array(rest.position);
                let delta = p.distance(Vec3::from_array(now.position));
                if p.x * sign > 0. && p.y < upper_y {
                    movement = movement.max(delta);
                }
                if p.x * sign < 0. || p.y > 0. {
                    assert!(delta < 1e-6, "joint {joint} moved unrelated vertex {p:?}");
                }
            }
            assert!(
                movement > minimum_motion,
                "joint {joint} did not articulate: {movement}"
            );
        }
    }
    #[test]
    fn full_cycle_surface_quality() {
        let asset = voxy_render::ObjAsset::parse(
            include_str!("/Users/themoretheless/Documents/ChatGPT/Voxy/assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let original = asset.mesh.vertices();
        let rig = FemaleRig::new(original).unwrap();
        let mut min_area = f32::INFINITY;
        let mut max_stretch = 0f32;
        let mut worst = (Vec3::ZERO, Vec3::ZERO);
        let mut worst_area = (0, [Vec3::ZERO;3]);
        for sample in 0..=24 {
            let mut posed = original.to_vec();
            rig.deform(&mut posed, sample as f32 * 0.25);
            for triangle in asset.mesh.indices().chunks_exact(3) {
                let rest: [Vec3; 3] = std::array::from_fn(|i| {
                    Vec3::from_array(original[triangle[i] as usize].position)
                });
                let now: [Vec3; 3] =
                    std::array::from_fn(|i| Vec3::from_array(posed[triangle[i] as usize].position));
                let area = (rest[1] - rest[0]).cross(rest[2] - rest[0]).length();
                if area > 1e-10 {
                    let ratio=(now[1]-now[0]).cross(now[2]-now[0]).length()/area;
                    if ratio<min_area {min_area=ratio;worst_area=(sample,rest);}
                }
                for i in 0..3 {
                    let j = (i + 1) % 3;
                    let length = rest[i].distance(rest[j]);
                    if length > 1e-6 {
                        let ratio = now[i].distance(now[j]) / length;
                        if ratio > max_stretch {
                            max_stretch = ratio;
                            worst = (rest[i], rest[j]);
                        }
                    }
                }
            }
        }
        println!(
            "RIG SURFACE AUDIT: min triangle area ratio={min_area:.4}; max edge stretch={max_stretch:.4}"
        );
        println!("worst edge: {worst:?}; worst area: {worst_area:?}");
        assert!(min_area > 0.7, "collapsed triangle: {min_area}");
        assert!(max_stretch < 1.35, "stretched edge: {max_stretch}");
    }
    #[test]
    fn actual_model_bind_pose_loop_and_arm_motion() {
        let asset = voxy_render::ObjAsset::parse(
            include_str!("/Users/themoretheless/Documents/ChatGPT/Voxy/assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let original = asset.mesh.vertices();
        let rig = FemaleRig::new(original).unwrap();
        assert_eq!(rig.skeleton.joints().len(), 54);
        assert!(
            rig.weights
                .iter()
                .all(|w| (w.iter().map(|x| x.1).sum::<f32>() - 1.).abs() < 1e-6)
        );
        for time in [0., 6.] {
            let mut vertices = original.to_vec();
            rig.deform(&mut vertices, time);
            assert!(vertices.iter().zip(original).all(|(a, b)| {
                Vec3::from_array(a.position).distance(Vec3::from_array(b.position)) < 1e-6
            }));
        }
        let mut animated = original.to_vec();
        rig.deform(&mut animated, 3.);
        let motion = animated
            .iter()
            .zip(original)
            .filter(|(_, b)| b.position[0].abs() > 0.32)
            .map(|(a, b)| Vec3::from_array(a.position).distance(Vec3::from_array(b.position)))
            .fold(0f32, f32::max);
        assert!(motion > 0.05, "arm movement {motion}");
        let torso_motion = animated
            .iter()
            .zip(original)
            .filter(|(_, v)| v.position[0].abs() < 0.12 && (0.15..0.45).contains(&v.position[1]))
            .map(|(a, b)| Vec3::from_array(a.position).distance(Vec3::from_array(b.position)))
            .fold(0_f32, f32::max);
        assert!(
            torso_motion > 0.003,
            "torso did not follow gesture: {torso_motion}"
        );
        assert!(
            animated
                .iter()
                .all(|v| Vec3::from_array(v.position).is_finite())
        );
    }
}

#[cfg(test)]
mod thumb_tip_contact_tests {
    use super::*;
    #[test]
    fn closed_thumb_tip_contacts_cylinder_and_sphere() {
        let asset=voxy_render::ObjAsset::parse(include_str!("/Users/themoretheless/Documents/ChatGPT/Voxy/assets/characters/blender-female/body.obj"),voxy_render::ObjLimits::default()).unwrap();
        let rest = asset.mesh.vertices();
        for object in [
            GraspObject::Cylinder {
                center: Vec3::new(0.350, -0.075, 0.078),
                radius: 0.024,
                half_length: 0.060,
            },
            GraspObject::Sphere {
                center: Vec3::new(0.350, -0.075, 0.078),
                radius: 0.032,
            },
        ] {
            let object = std::sync::Arc::new(object);
            let mut rig = FemaleRig::new(rest).unwrap();
            rig.bind_hand_surface(rest, asset.mesh.indices()).unwrap();
            rig.set_body_motion(false);
            rig.set_grasp_object(Some(object.clone()));
            rig.set_grasp(1.).unwrap();
            let mut posed = rest.to_vec();
            rig.deform(&mut posed, 0.);
            for side in 0..2 {
                let bone = 26 + side * 15;
                let mirror = Vec3::new(if side == 0 { 1. } else { -1. }, 1., 1.);
                let inv = rig.hand_matrix(0., side == 1).inverse();
                let mut count = 0;
                let mut gap = f32::INFINITY;
                for (i, v) in posed.iter().enumerate() {
                    if rig.finger_weights[i]
                        .as_ref()
                        .is_some_and(|ws| ws.iter().any(|(b, w)| *b == bone && *w >= 0.8))
                    {
                        count += 1;
                        gap =
                            gap.min(object.distance(
                                inv.transform_point3(Vec3::from_array(v.position)) * mirror,
                            ));
                    }
                }
                assert!(count > 0, "thumb tip binding is missing for side {side}");
                assert!(
                    (-0.000001..=0.002).contains(&gap),
                    "thumb tip lost contact with {object:?}, side {side}: {gap} m"
                );
            }
        }
    }
}

#[cfg(test)]mod metacarpal_export {use super::*;
#[test]fn export_reference(){use std::fmt::Write;
let asset=voxy_render::ObjAsset::parse(include_str!("/Users/themoretheless/Documents/ChatGPT/Voxy/assets/characters/blender-female/body.obj"),voxy_render::ObjLimits::default()).unwrap();let original=asset.mesh.vertices();
for amount in [0.,0.5,1.] {let mut rig=FemaleRig::new(original).unwrap();assert_eq!(rig.skeleton.joints().len(),54);let mut tracks=vec![JointTrack::default();54];
for side in 0..2{let sign=if side==0{1.}else{-1.};for finger in 0..5{for j in 0..3{
let axes=FingerJointAxes::for_segment(FINGER_CHAINS[finger],j,sign);let flex=if finger==0{[0.20,0.65,0.45]}else{[1.10,1.50,0.70]};let spread=if finger==0 && j==0{0.70}else{0.};let twist=if finger==0 && j==0{0.20}else{0.};tracks[24+side*15+finger*3+j].rotations=vec![QuatKey{time:0.,value:axes.rotation(flex[j]*amount,spread*amount,twist*amount)}];}
if finger>0{let z=[0.070,0.055,0.040,0.025][finger-1];let a=Vec3::new(sign*0.35,0.005,z);let b=Vec3::new(sign*FINGER_CHAINS[finger][0][0],FINGER_CHAINS[finger][0][1],FINGER_CHAINS[finger][0][2]);let axis=(b-a).cross(Vec3::new(-sign,0.,0.)).normalize();tracks[16+side*4+finger-1].rotations=vec![QuatKey{time:0.,value:Quat::from_axis_angle(axis,[0.,0.06,0.16,0.26][finger-1]*amount)}];}
}}
rig.clip=AnimationClip::new("metacarpal prototype",1.,Playback::Clamp,tracks,&rig.skeleton).unwrap();let mut posed=original.to_vec();rig.deform(&mut posed,0.);
for w in rig.finger_weights.iter().flatten(){assert!((w.iter().map(|(_,v)|v).sum::<f32>()-1.).abs()<1e-5);assert!(w.iter().all(|(b,v)|*b<54 && *v>=0.));}
for side in 0..2{let sign=if side==0{1.}else{-1.};let mut csv=String::from("rest_x,rest_y,rest_z,posed_x,posed_y,posed_z\n");for(a,b)in original.iter().zip(&posed){let p=a.position;if p[0]*sign>0.32 && p[1]>-0.16 && p[1]<0.03{writeln!(csv,"{},{},{},{},{},{}",p[0]*sign,p[1],p[2],b.position[0]*sign,b.position[1],b.position[2]).unwrap();}else if p[0].abs()<0.30{assert!(Vec3::from_array(a.position).distance(Vec3::from_array(b.position))<1e-6);}}
std::fs::write(format!("target/hand-metacarpal-engine-audit/finger-0-side-{side}-amount-{amount}.csv"),csv).unwrap();}
}
}}

#[cfg(test)]mod arap_diagnostic {use super::*;
#[test]fn export_arap(){use std::fmt::Write;
let asset=voxy_render::ObjAsset::parse(include_str!("/Users/themoretheless/Documents/ChatGPT/Voxy/assets/characters/blender-female/body.obj"),voxy_render::ObjLimits::default()).unwrap();let rest=asset.mesh.vertices();let n=rest.len();
let points:Vec<Vec3>=rest.iter().map(|v|Vec3::from_array(v.position)).collect();
let mut rig=FemaleRig::new(rest).unwrap();rig.set_body_motion(false);
let mask:Vec<f32>=points.iter().enumerate().map(|(i,p)|rig.finger_weights[i].as_ref().map_or(0.,|weights|{let dominant=weights.iter().map(|(_,w)|*w).fold(0.,f32::max);f32::max(1.-smooth(0.98,0.999,dominant),smooth(0.065,0.075,p.z)*(1.-smooth(0.105,0.12,p.z))*(1.-smooth(0.37,0.385,p.x.abs())))*smooth(0.32,0.345,p.x.abs())*(1.-smooth(0.01,0.03,p.y))})).collect();
let mut edges=std::collections::BTreeSet::new();for f in asset.mesh.indices().chunks_exact(3){for j in 0..3{let a=f[j]as usize;let b=f[(j+1)%3]as usize;if mask[a]>0. || mask[b]>0.{edges.insert((a.min(b),a.max(b)));}}}
let mut neighbors=vec![Vec::<(usize,f32)>::new();n];for(a,b)in edges{let w=1./points[a].distance_squared(points[b]).max(1e-10);neighbors[a].push((b,w));neighbors[b].push((a,w));}
let active:Vec<usize>=(0..n).filter(|&i|mask[i]>0.).collect();println!("ARAP active vertices {}",active.len());
std::fs::create_dir_all("target/hand-local-global-diagnostic").unwrap();
for amount in [0.,0.25,0.50,0.75,1.]{rig.set_grasp(amount).unwrap();let mut posed=rest.to_vec();rig.deform(&mut posed,0.);let raw:Vec<Vec3>=posed.iter().map(|v|Vec3::from_array(v.position)).collect();
let palette:Vec<_>=rig.pose(0.).skin_matrices(&rig.skeleton).unwrap().into_iter().map(crate::rig_skinning::RigidSkinTransform::from_matrix).collect();
let mut rotations=vec![[Vec3::X,Vec3::Y,Vec3::Z];n];for i in 0..n{if let Some(w)=&rig.finger_weights[i]{let origin=crate::rig_skinning::deform_point(Vec3::ZERO,w,&palette);rotations[i]=[Vec3::X,Vec3::Y,Vec3::Z].map(|axis|crate::rig_skinning::deform_point(axis,w,&palette)-origin);}}
let mut solved=raw.clone();let mut residual=0.;
for outer in 0..20 {
if outer>0 {for &i in &active {let mut covariance=[Vec3::ZERO;3];for &(j,w) in &neighbors[i]{let e=points[i]-points[j];let d=solved[i]-solved[j];for k in 0..3{covariance[k]+=d*(e[k]*w);}}let mut q=Quat::from_mat3(&glam::Mat3::from_cols(rotations[i][0],rotations[i][1],rotations[i][2])).normalize();for _ in 0..30 {let basis=[q*Vec3::X,q*Vec3::Y,q*Vec3::Z];let numerator=basis[0].cross(covariance[0])+basis[1].cross(covariance[1])+basis[2].cross(covariance[2]);let denominator=(basis[0].dot(covariance[0])+basis[1].dot(covariance[1])+basis[2].dot(covariance[2])).abs()+1e-9;let omega=numerator/denominator;let angle=omega.length();if angle<1e-6{break;}q=(Quat::from_axis_angle(omega/angle,angle)*q).normalize();}rotations[i]=[q*Vec3::X,q*Vec3::Y,q*Vec3::Z];}}
let mut rhs=vec![glam::DVec3::ZERO;n];let mut diagonal=vec![0f64;n];
for &i in &active{let sum: f64=neighbors[i].iter().map(|(_,w)|*w as f64).sum();let anchor=sum*(0.0001+(1.-mask[i] as f64).powi(2)*10.);diagonal[i]=sum+anchor;rhs[i]=raw[i].as_dvec3()*anchor;for &(j,w)in &neighbors[i]{let edge=points[i].as_dvec3()-points[j].as_dvec3();let rd=|k:usize|rotations[k][0].as_dvec3()*edge.x+rotations[k][1].as_dvec3()*edge.y+rotations[k][2].as_dvec3()*edge.z;rhs[i]+=(rd(i)+rd(j))*(w as f64*0.5);}}
residual=0.;
for axis in 0..3 {
 let mut x:Vec<f64>=solved.iter().map(|p|p[axis] as f64).collect();
 let mut b=vec![0f64;n];
 for &i in &active {b[i]=rhs[i][axis] as f64;for &(j,w) in &neighbors[i]{if mask[j]<=0. {b[i]+=w as f64*x[j];}}}
 let apply=|x:&[f64]| {let mut out=vec![0f64;n];for &i in &active {out[i]=diagonal[i] as f64*x[i];for &(j,w) in &neighbors[i]{if mask[j]>0. {out[i]-=w as f64*x[j];}}}out};
 let ax=apply(&x);let mut r=vec![0f64;n];let mut z=vec![0f64;n];for &i in &active{r[i]=b[i]-ax[i];z[i]=r[i]/diagonal[i] as f64;}
 let mut direction=z.clone();let dot=|a:&[f64],b:&[f64]|active.iter().map(|&i|a[i]*b[i]).sum::<f64>();let mut rz=dot(&r,&z);
 for iteration in 0..2000 {let ad=apply(&direction);let denominator=dot(&direction,&ad);if denominator.abs()<1e-30 {break;}let alpha=rz/denominator;for &i in &active{x[i]+=alpha*direction[i];r[i]-=alpha*ad[i];}let error=active.iter().map(|&i|(r[i]/diagonal[i] as f64).abs()).fold(0f64,f64::max);if error<1e-10 {println!("PCG {amount} axis {axis}: {iteration} iterations, normalized residual {error}");break;}for &i in &active{z[i]=r[i]/diagonal[i] as f64;}let next=dot(&r,&z);let beta=next/rz;for &i in &active{direction[i]=z[i]+beta*direction[i];}rz=next;}
 let ax=apply(&x);for &i in &active{residual=f32::max(residual,((b[i]-ax[i])/diagonal[i] as f64).abs() as f32);solved[i][axis]=x[i] as f32;}
}
}
println!("ARAP {amount}: final residual {residual}, max delta {}",active.iter().map(|&i|solved[i].distance(raw[i])).fold(0.,f32::max));
for mode in ["raw","arap"]{let p=if mode=="raw"{&raw}else{&solved};let mut csv=String::from("rest_x,rest_y,rest_z,posed_x,posed_y,posed_z\n");for i in 0..n{let r=points[i];if r.x>0.32 && r.y>-0.16 && r.y<0.03{writeln!(csv,"{},{},{},{},{},{}",r.x,r.y,r.z,p[i].x,p[i].y,p[i].z).unwrap();}}std::fs::write(format!("target/hand-local-global-diagnostic/finger-0-{mode}-{amount}.csv"),csv).unwrap();}
}}
}
