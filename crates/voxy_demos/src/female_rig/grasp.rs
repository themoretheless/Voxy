//! Hand grasp geometry, swept contact fitting, and anatomical finger/body skin weights.
use super::*;

/// Object in the canonical left hand's rest frame. Meshes must be closed and outward wound.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum GraspObject {
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
    pub fn from_mesh(vertices: &[SceneVertex], indices: &[u32]) -> Result<Self, &'static str> {
        if vertices.is_empty()
            || indices.is_empty()
            || !indices.len().is_multiple_of(3)
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
        let mut faces = Vec::with_capacity(indices.len() / 3);
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
        let mut pending = Vec::new();
        for (vertex, fan) in incident.iter().enumerate() {
            if fan.is_empty() {
                continue;
            }
            if fan.len() <= 64 {
                let mut seen_mask = 1u64;
                pending.clear();
                pending.push(fan[0]);
                while let Some(id) = pending.pop() {
                    for &neighbor in &adjacency[id] {
                        if faces[neighbor].contains(&vertex)
                            && let Some(pos) = fan.iter().position(|&f| f == neighbor)
                        {
                            let bit = 1u64 << pos;
                            if seen_mask & bit == 0 {
                                seen_mask |= bit;
                                pending.push(neighbor);
                            }
                        }
                    }
                }
                if seen_mask.count_ones() as usize != fan.len() {
                    return Err("nonmanifold grasp mesh vertex");
                }
            } else {
                let mut seen = std::collections::BTreeSet::new();
                pending.clear();
                pending.push(fan[0]);
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
    pub(super) fn closest_point(&self, point: Vec3) -> Vec3 {
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
    pub(super) fn distance(&self, point: Vec3) -> f32 {
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
                // Ambiguous ray hits retain the solid-angle winding classifier.
                let inside = mesh
                    .contains_closed_surface(point.to_array().map(f64::from))
                    .expect("finite hand position")
                    .unwrap_or_else(|| {
                        triangles
                            .iter()
                            .map(|&face| solid_angle(face, point))
                            .sum::<f64>()
                            .abs()
                            > std::f64::consts::TAU
                    });
                delta.length() * if inside { -1. } else { 1. }
            }
        }
    }
}

pub(super) fn solid_angle(face: [Vec3; 3], point: Vec3) -> f64 {
    let [a, b, c] = face.map(|v| (v - point).as_dvec3());
    let numerator = a.dot(b.cross(c));
    let denominator = a.length() * b.length() * c.length()
        + a.dot(b) * c.length()
        + b.dot(c) * a.length()
        + c.dot(a) * b.length();
    2. * numerator.atan2(denominator)
}

/// Restrict weights to anatomically connected regions: a nearby hand cannot pull the hip,
/// and the opposite arm cannot influence this arm. Joint bands blend adjacent bones only.
// Imported mannequin finger centerlines, in the same metre/Y-up rest space as body.obj.
// Thumb plus index, middle, ring and little finger; three articulated segments each.
pub(super) const FINGER_CHAINS: [[[f32; 3]; 4]; 5] = [
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
pub(super) struct FingerJointAxes {
    pub(super) flexion: Vec3,
    pub(super) spread: Vec3,
    pub(super) twist: Vec3,
}
impl FingerJointAxes {
    pub(super) fn for_segment(chain: [[f32; 3]; 4], segment: usize, side: f32) -> Self {
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
    pub(super) fn rotation(self, flexion: f32, spread: f32, twist: f32) -> Quat {
        Quat::from_axis_angle(self.twist, twist)
            * Quat::from_axis_angle(self.spread, spread)
            * Quat::from_axis_angle(self.flexion, flexion)
    }
}
pub(super) fn finger_clearance(
    chain: [[f32; 3]; 4],
    rotations: [Quat; 3],
    object: &GraspObject,
) -> f32 {
    finger_clearance_from(chain, rotations, object, 0)
}
pub(super) fn finger_clearance_from(
    chain: [[f32; 3]; 4],
    rotations: [Quat; 3],
    object: &GraspObject,
    first: usize,
) -> f32 {
    if let Some(finger) = FINGER_CHAINS
        .iter()
        .position(|candidate| *candidate == chain)
        && (finger != 0 || !matches!(object, GraspObject::Cylinder { .. }))
    {
        // Use both actual hand surfaces: their vertices and skin weights differ.
        // Retain a small clearance margin before the final surface correction.
        type Cloud = Vec<(Vec3, Box<[(usize, f32)]>)>;
        static CLOUDS: std::sync::OnceLock<[[Cloud; 3]; 5]> = std::sync::OnceLock::new();
        let clouds = CLOUDS.get_or_init(|| {
            let asset = voxy_render::ObjAsset::parse(
                include_str!("../../../../assets/characters/blender-female/body.obj"),
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
                    let root = 16 + side * 15 + finger * 3;
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
                        .filter(|(bone, weight)| (root..root + 3).contains(bone) && *weight > 0.)
                        .map(|(bone, weight)| (*bone - side * 15, *weight))
                        .collect();
                    compact.push((0, 1. - own));
                    for first in 0..3 {
                        if compact.iter().any(|(bone, weight)| {
                            (16 + finger * 3 + first..16 + finger * 3 + 3).contains(bone)
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
            return finger_surface_gap(chain, finger, rotations, object, &clouds[finger][first])
                - if finger == 0 { 0.0002 } else { 0.0001 };
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
pub(super) fn prepare_finger_grasp(
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

pub(super) fn fit_finger(
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

pub(super) fn curled_thumb_contact_path(
    reference: &[[Quat; 3]],
    object: &GraspObject,
    skin: &[(Vec3, Box<[(usize, f32)]>)],
) -> Option<Vec<[Quat; 3]>> {
    let axes: [Vec3; 3] = std::array::from_fn(|j| {
        // Existing reference poses use extension here. Keep the signed mapping
        // explicit until those poses are replaced; extension is not flexion.
        -FingerJointAxes::for_segment(FINGER_CHAINS[0], j, 1.).flexion
    });
    // Require contact on the distal thumb surface, rather than any phalanx.
    let pad: Vec<_> = skin
        .iter()
        .filter(|(_, weights)| weights.iter().any(|(bone, w)| *bone == 18 && *w >= 0.80))
        .cloned()
        .collect();
    if pad.is_empty() {
        return None;
    }
    let mut candidates = Vec::new();
    let mesh = matches!(object, GraspObject::Mesh { .. });
    let x_limit = if mesh { 40 } else { 25 };
    let y_limit = if mesh { 30 } else { 15 };
    for x in -x_limit..=x_limit {
        for y in -y_limit..=y_limit {
            let root = Quat::from_rotation_y(y as f32 * 0.02)
                * Quat::from_rotation_x(x as f32 * 0.02)
                * reference[120][0];
            let pose = [
                root,
                Quat::from_axis_angle(axes[1], if mesh { 0.35 } else { 0.50 }),
                Quat::from_axis_angle(axes[2], if mesh { 0.22 } else { 0.30 }),
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

pub(super) fn cylinder_thumb_contact_path(
    reference: &[[Quat; 3]],
    object: &GraspObject,
    skin: &[(Vec3, Box<[(usize, f32)]>)],
) -> Option<Vec<[Quat; 3]>> {
    if skin.is_empty() {
        return None;
    }
    let closed = reference[120];
    let axes: [Vec3; 3] = std::array::from_fn(|j| {
        // Existing reference poses use extension here. Keep the signed mapping
        // explicit until those poses are replaced; extension is not flexion.
        -FingerJointAxes::for_segment(FINGER_CHAINS[0], j, 1.).flexion
    });
    let mut target = [
        Quat::from_rotation_y(-0.25) * Quat::from_rotation_x(0.17) * closed[0],
        closed[1] * Quat::from_axis_angle(axes[1], 0.10),
        closed[2] * Quat::from_axis_angle(axes[2], 0.10),
    ];
    let moderate = projected_thumb_contact_path(target, object, skin, 0.012)?;
    let longitudinal =
        (Vec3::from_array(FINGER_CHAINS[0][1]) - Vec3::from_array(FINGER_CHAINS[0][0])).normalize();
    target = [
        Quat::from_rotation_y(0.39)
            * Quat::from_rotation_x(0.47)
            * moderate[120][0]
            * Quat::from_axis_angle(longitudinal, 0.60),
        Quat::from_axis_angle(axes[1], 0.75),
        Quat::from_axis_angle(axes[2], 0.55),
    ];
    let strong = projected_thumb_contact_path(target, object, skin, 0.0124).unwrap_or(moderate);
    target = [
        Quat::from_rotation_y(0.10)
            * Quat::from_rotation_x(-0.50)
            * strong[120][0]
            * Quat::from_axis_angle(longitudinal, 0.50),
        Quat::from_axis_angle(-axes[1], 0.75),
        Quat::from_axis_angle(-axes[2], 0.55),
    ];
    projected_thumb_contact_path(target, object, skin, 0.0124).or(Some(strong))
}

pub(super) fn projected_thumb_contact_path(
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

pub(super) fn finger_surface_gap(
    chain: [[f32; 3]; 4],
    finger: usize,
    pose: [Quat; 3],
    object: &GraspObject,
    skin: &[(Vec3, Box<[(usize, f32)]>)],
) -> f32 {
    let mut palette =
        vec![crate::rig_skinning::RigidSkinTransform::from_matrix(Mat4::IDENTITY); 46];
    let mut transform = Mat4::IDENTITY;
    for joint in 0..3 {
        let origin = Vec3::from_array(chain[joint]);
        let parent = if joint == 0 {
            Vec3::ZERO
        } else {
            Vec3::from_array(chain[joint - 1])
        };
        transform *= Mat4::from_translation(origin - parent) * Mat4::from_quat(pose[joint]);
        palette[16 + finger * 3 + joint] = crate::rig_skinning::RigidSkinTransform::from_matrix(
            transform * Mat4::from_translation(-origin),
        );
    }

    skin.iter()
        .map(|(point, weights)| {
            object.distance(crate::rig_skinning::deform_point(*point, weights, &palette))
        })
        .fold(f32::INFINITY, f32::min)
}

pub(super) fn cylinder_curled_contact_pose(
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
                .any(|(bone, weight)| *bone == 18 + finger * 3 && *weight >= 0.8)
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

pub(super) fn grasp_rotation(finger: usize, segment: usize, sign: f32) -> Quat {
    let axes = FingerJointAxes::for_segment(FINGER_CHAINS[finger], segment, sign);
    // The legacy thumb reference extends its phalanges. Its sign is explicit;
    // new anatomical controls use axes.flexion with positive flexion angles.
    let flex = if finger == 0 {
        [0.10, 0.40, 0.25]
    } else {
        [0.60, 0.95, 0.55]
    };
    let signed_flexion = if finger == 0 {
        -flex[segment]
    } else {
        flex[segment]
    };
    let bend = axes.rotation(signed_flexion, 0., 0.);
    if segment != 0 {
        return bend;
    }
    if finger == 0 {
        // Opposition brings the thumb across the palm toward the index finger.
        Quat::from_rotation_y(-sign * 0.35) * Quat::from_rotation_x(0.65) * bend
    } else {
        bend
    }
}
pub(super) fn finger_weights(p: Vec3) -> Option<Vec<(usize, f32)>> {
    let mut weights = finger_weights_uncorrected(p)?;
    // Baked against both actual hands and sixteen grasp poses. Match rest positions,
    // so OBJ vertex reordering does not change the binding.
    static CORRECTIONS: std::sync::OnceLock<std::collections::BTreeMap<[i32; 3], f32>> =
        std::sync::OnceLock::new();
    let corrections = CORRECTIONS.get_or_init(|| {
        include_str!("../../../../assets/characters/blender-female/thumb-bind-correction.csv")
            .lines()
            .map(|line| {
                let values: Vec<f32> = line
                    .split(',')
                    .map(|v| v.parse().expect("valid baked thumb binding"))
                    .collect();
                (
                    std::array::from_fn(|i| (values[i] * 1_000_000.).round() as i32),
                    values[3],
                )
            })
            .collect()
    });
    let key = p.to_array().map(|v| (v * 1_000_000.).round() as i32);
    if let Some(&delta) = corrections.get(&key) {
        let root = if p.x >= 0. { 16 } else { 31 };
        let thumb = weights.iter().position(|(bone, _)| *bone == root).unwrap();
        let palm = weights.iter().position(|(bone, _)| *bone == 0).unwrap();
        let delta = delta.clamp(-weights[thumb].1, weights[palm].1);
        weights[thumb].1 += delta;
        weights[palm].1 -= delta;
    }
    Some(weights)
}
pub(super) fn finger_weights_uncorrected(p: Vec3) -> Option<Vec<(usize, f32)>> {
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
    let side = usize::from(p.x < 0.);
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
        let start = 16 + side * 15 + finger * 3;
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
pub(super) fn anatomical_weights(p: Vec3) -> [(usize, f32); 4] {
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
