//! Surface-attached brow/lash/fuzz strands and animated oral geometry.
#![allow(clippy::cast_precision_loss)] // Small fixed tessellation counts fit f32 exactly.
use glam::{Mat3, Mat4, Quat, Vec3};
use voxy_render::SceneVertex;

#[derive(Clone, Copy, Debug)]
enum Kind {
    Brow,
    Lash,
    LowerLash,
    Fuzz,
}
#[derive(Clone, Copy, Debug)]
struct Strand {
    roots: [usize; 3],
    weights: Vec3,
    direction: Vec3,
    radius: f32,
    kind: Kind,
    side: f32,
    bind_frame: Quat,
}
#[derive(Debug)]
pub(crate) struct FaceFeatures {
    strands: Vec<Strand>,
    lash_motion: Vec<physics::secondary_motion::SecondaryMotion>,
    seams: Vec<([u32; 3], [Vec3; 3])>,
    rims: Vec<Vec<([usize; 3], Vec3)>>,
    lash_skin_vertices: Vec<usize>,
    lash_skin_triangles: Vec<[usize; 3]>,
}
impl FaceFeatures {
    pub(crate) fn advance_lashes(
        &mut self,
        dt: f64,
        acceleration: [f64; 3],
    ) -> Result<(), &'static str> {
        for (strand, motion) in self.strands.iter().zip(&mut self.lash_motion) {
            if matches!(strand.kind, Kind::Lash | Kind::LowerLash) {
                motion.step(dt, acceleration, [0.; 3])?;
            }
        }
        Ok(())
    }
    pub fn new(body: &[SceneVertex], indices: &[u32]) -> Self {
        let face: Vec<[usize; 3]> = indices
            .chunks_exact(3)
            .filter_map(|ids| {
                let ids: [usize; 3] = std::array::from_fn(|i| ids[i] as usize);
                if ids.iter().any(|&i| i >= body.len()) {
                    return None;
                }
                let center = ids
                    .iter()
                    .map(|&i| Vec3::from_array(body[i].position))
                    .sum::<Vec3>()
                    / 3.;
                (center.y > 0.62 && center.y < 0.79 && center.z > 0.105 && center.x.abs() < 0.09)
                    .then_some(ids)
            })
            .collect();
        let lid_contour = crate::female_face::LidContour::new(body, indices);
        let mut strands = Vec::new();
        let mut add = |point: Vec3, direction: Vec3, radius: f32, kind| {
            let (roots, weights, _) = face
                .iter()
                .map(|&ids| {
                    let triangle = ids.map(|i| Vec3::from_array(body[i].position));
                    // Preserve aperture XY placement while selecting its front surface.
                    let metric = if matches!(kind, Kind::Lash | Kind::LowerLash) {
                        Vec3::new(1., 1., 0.25)
                    } else {
                        Vec3::ONE
                    };
                    let weights =
                        closest_triangle_weights(point * metric, triangle.map(|p| p * metric));
                    let projected =
                        triangle[0] * weights.x + triangle[1] * weights.y + triangle[2] * weights.z;
                    (
                        ids,
                        weights,
                        ((point - projected) * metric).length_squared(),
                    )
                })
                .min_by(|a, b| a.2.total_cmp(&b.2))
                .expect("actual facial surface");
            strands.push(Strand {
                roots,
                weights,
                direction,
                radius,
                kind,
                side: point.x.signum(),
                bind_frame: skin_frame(roots.map(|i| Vec3::from_array(body[i].position)))
                    .unwrap_or(Quat::IDENTITY),
            });
        };
        for side in [-1., 1.] {
            for i in 0..600 {
                let t = i as f32 / 599.;
                let x = 0.013 + 0.048 * t;
                let y = 0.737
                    + 0.009 * (t * std::f32::consts::PI).sin()
                    + (((i * 73) % 101) as f32 / 100. - 0.5) * 0.007;
                let z = 0.144 - 0.035 * t;
                add(
                    Vec3::new(side * x, y, z),
                    Vec3::new(side * 0.0035, 0.0025 * (1. - t) + 0.0005, 0.0007),
                    0.000075,
                    Kind::Brow,
                );
            }
            for i in 0..48 {
                // Stratified roots preserve ordering while avoiding a comb-like row.
                // Each side has a different deterministic groom.
                let seed = i + if side > 0. { 193 } else { 71 };
                let jitter = groom_noise(seed);
                let length = 0.72 + 0.52 * groom_noise(seed + 409);
                let t = (i as f32 + 0.7 * (jitter - 0.5)).clamp(0., 47.) / 47.;
                let x = 0.019 + 0.028 * t;
                let arc = (t * std::f32::consts::PI).sin();
                add(
                    Vec3::new(
                        side * x,
                        lid_contour.aperture_at(side * x)[1] + 0.0002,
                        0.145,
                    ),
                    Vec3::new(
                        side * (-0.0004 + 0.003 * t + 0.0012 * (jitter - 0.5)),
                        (0.003 + 0.0022 * arc) * length,
                        (0.0028 + 0.001 * arc) * length,
                    ),
                    0.000035 + 0.000018 * groom_noise(seed + 811),
                    Kind::Lash,
                );
            }
            for i in 0..24 {
                let seed = i + if side > 0. { 2101 } else { 2503 };
                let jitter = groom_noise(seed);
                let t = (i as f32 + 0.9 * (jitter - 0.5)).clamp(0., 23.) / 23.;
                let arc = (t * std::f32::consts::PI).sin();
                let length = 0.65 + 0.65 * groom_noise(seed + 409);
                add(
                    Vec3::new(
                        side * (0.019 + 0.028 * t),
                        lid_contour.aperture_at(side * (0.019 + 0.028 * t))[0] - 0.00015,
                        0.145,
                    ),
                    Vec3::new(
                        side * (0.0002 + 0.0012 * t),
                        -(0.0012 + 0.0016 * arc) * length,
                        (0.0012 + 0.0005 * arc) * length,
                    ),
                    0.000024 + 0.000012 * groom_noise(seed + 811),
                    Kind::LowerLash,
                );
            }
            for i in 0..110 {
                let t = i as f32 / 109.;
                let x = side * (0.05 + 0.016 * ((i * 37 % 101) as f32 / 100.));
                let y = 0.649 + 0.057 * t;
                let z = 0.127 - 0.013 * t;
                add(
                    Vec3::new(x, y, z),
                    Vec3::new(side * 0.0003, -0.0009, 0.0005),
                    0.000_035,
                    Kind::Fuzz,
                );
            }
        }
        let seams = indices
            .chunks_exact(3)
            .filter_map(|ids| {
                if ids.iter().any(|&i| i as usize >= body.len()) {
                    return None;
                }
                let rest =
                    std::array::from_fn(|i| Vec3::from_array(body[ids[i] as usize].position));
                is_seal(rest).then_some(([ids[0], ids[1], ids[2]], rest))
            })
            .collect();
        let mut rims = Vec::new();
        for side in [-1., 1.] {
            for upper in [false, true] {
                let mut rim = Vec::new();
                for i in 0..=32 {
                    let t = i as f32 / 32.;
                    let arc = (t * std::f32::consts::PI).sin();
                    let point = Vec3::new(
                        side * (0.019 + 0.028 * t),
                        if upper {
                            0.712 + 0.006 * arc
                        } else {
                            0.710 - 0.004 * arc
                        },
                        0.130,
                    );
                    let (ids, weights, _) = face
                        .iter()
                        .map(|&ids| {
                            let triangle = ids.map(|j| Vec3::from_array(body[j].position));
                            let weights = closest_triangle_weights(point, triangle);
                            let projected = triangle[0] * weights.x
                                + triangle[1] * weights.y
                                + triangle[2] * weights.z;
                            (ids, weights, point.distance_squared(projected))
                        })
                        .min_by(|a, b| a.2.total_cmp(&b.2))
                        .expect("lid surface");
                    rim.push((ids, weights));
                }
                rims.push(rim);
            }
        }
        let contact_faces: Vec<_> = face
            .iter()
            .copied()
            .filter(|ids| {
                let [a, b, c] = ids.map(|i| Vec3::from_array(body[i].position));
                let center = (a + b + c) / 3.;
                center.y > 0.685 && center.y < 0.74 && (b - a).cross(c - a).z > 0.
            })
            .collect();
        let lash_skin_vertices: Vec<_> = contact_faces
            .iter()
            .flatten()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        let mapping: std::collections::BTreeMap<_, _> = lash_skin_vertices
            .iter()
            .enumerate()
            .map(|(i, &v)| (v, i))
            .collect();
        let lash_skin_triangles = contact_faces
            .iter()
            .map(|ids| ids.map(|i| mapping[&i]))
            .collect();
        Self {
            lash_motion: strands
                .iter()
                .map(|_| {
                    physics::secondary_motion::SecondaryMotion::new(
                        physics::secondary_motion::Config {
                            frequency: 24.,
                            damping_ratio: 0.3,
                        },
                    )
                    .unwrap()
                })
                .collect(),
            strands,
            seams,
            rims,
            lash_skin_vertices,
            lash_skin_triangles,
        }
    }
    #[allow(clippy::too_many_arguments)] // Surface attachment inputs and existing render buffers.
    pub fn append(
        &self,
        body: &[SceneVertex],
        normals: &[Vec3],
        vertices: &mut Vec<SceneVertex>,
        indices: &mut Vec<u32>,
        head: Mat4,
        pose: crate::female_face::FacePose,
    ) {
        self.append_configured(
            body,
            normals,
            vertices,
            indices,
            head,
            pose,
            &crate::face_parameters::FaceParameters::default(),
        );
    }
    #[allow(clippy::too_many_arguments)]
    pub fn append_configured(
        &self,
        body: &[SceneVertex],
        normals: &[Vec3],
        vertices: &mut Vec<SceneVertex>,
        indices: &mut Vec<u32>,
        head: Mat4,
        pose: crate::female_face::FacePose,
        parameters: &crate::face_parameters::FaceParameters,
    ) {
        let contact = if std::env::var("VOXY_FACE_DIAGNOSTIC_NO_LASH_CONTACT").as_deref() == Ok("1")
        {
            None
        } else {
            let points: Vec<_> = self
                .lash_skin_vertices
                .iter()
                .map(|&i| body[i].position.map(f64::from))
                .collect();
            physics::hair::TriangleMesh::new(&points, &self.lash_skin_triangles).ok()
        };
        let forward = head.transform_vector3(Vec3::Z).normalize();
        let contact_normals: Vec<_> = self
            .lash_skin_vertices
            .iter()
            .map(|&i| normals[i].to_array().map(f64::from))
            .collect();
        let omit_lashes = std::env::var("VOXY_FACE_DIAGNOSTIC_NO_LASHES").as_deref() == Ok("1");
        let omit_rims = std::env::var("VOXY_FACE_DIAGNOSTIC_NO_RIMS").as_deref() == Ok("1");
        for (index, strand) in self.strands.iter().enumerate() {
            if matches!(strand.kind, Kind::Lash | Kind::LowerLash) && omit_lashes {
                continue;
            }
            let prefix = match strand.kind {
                Kind::Brow => "brow_hair",
                Kind::Lash | Kind::LowerLash => "lashes",
                Kind::Fuzz => "",
            };
            let control = |suffix: &str| {
                parameters
                    .value(&format!("{prefix}_{suffix}"))
                    .unwrap_or(1.)
            };
            let visible =
                control("density") >= 1. || ((index * 73 % 101) as f32 / 100.) < control("density");
            let mut n = Vec3::ZERO;
            let mut root = Vec3::ZERO;
            for component in 0..3 {
                let index = strand.roots[component];
                n += normals[index] * strand.weights[component];
                root += Vec3::from_array(body[index].position) * strand.weights[component];
            }
            let n = n.try_normalize().unwrap_or(Vec3::Z);
            let root = root + n * 0.00008;
            let blink = pose.blink_for_side(strand.side);
            let lower = matches!(strand.kind, Kind::LowerLash);
            let fallback_rotation = if lower {
                Quat::from_rotation_x(-0.35 * blink.clamp(0., 1.))
            } else {
                lash_rotation(blink)
            };
            // Transport the complete lash shape with its attachment triangle.
            // The posed frame already includes blink, rig and head movement.
            let rotation = skin_frame(strand.roots.map(|i| Vec3::from_array(body[i].position)))
                .map(|frame| {
                    let surface = frame * strand.bind_frame.conjugate();
                    let hinge = Quat::from_mat4(&head) * fallback_rotation;
                    if std::env::var("VOXY_FACE_DIAGNOSTIC_LASH_HINGE_ONLY").as_deref() == Ok("1") {
                        hinge
                    } else {
                        constrain_lash_rotation(surface, hinge, blink)
                    }
                });
            let transport = |vector: Vec3| {
                rotation.map_or_else(
                    || head.transform_vector3(fallback_rotation * vector),
                    |frame| frame * vector,
                )
            };
            let mut direction = strand.direction * control("length");
            if matches!(strand.kind, Kind::Lash | Kind::LowerLash) {
                direction = transport(direction);
            } else {
                direction = head.transform_vector3(direction);
            }
            let mut color = match strand.kind {
                Kind::Brow => [0.065, 0.038, 0.022, 1.],
                Kind::Lash | Kind::LowerLash => [0.038, 0.025, 0.02, 1.],
                Kind::Fuzz => [0.43, 0.31, 0.21, 1.],
            };
            if !visible {
                color[3] = 0.;
            }
            if matches!(strand.kind, Kind::Lash | Kind::LowerLash) {
                let mut bend =
                    transport(lash_bend(strand.direction, lower, index)) * control("length");
                if let Some(motion) = self.lash_motion.get(index) {
                    // Root stays attached; inertial bending is resolved before existing skin contacts.
                    bend +=
                        head.transform_vector3(Vec3::from_array(motion.offset().map(|v| v as f32)));
                }
                curved_lash(
                    vertices,
                    indices,
                    root,
                    direction,
                    bend,
                    contact
                        .as_ref()
                        .map(|skin| (skin, forward, contact_normals.as_slice())),
                    strand.radius * control("thickness"),
                    color,
                );
            } else {
                tube(
                    vertices,
                    indices,
                    root,
                    root + direction,
                    strand.radius * control("thickness"),
                    color,
                );
            }
        }
        for rim in &self.rims {
            if omit_rims {
                continue;
            }
            let points: Vec<_> = rim
                .iter()
                .map(|&(ids, weights)| {
                    let mut point = Vec3::ZERO;
                    let mut normal = Vec3::ZERO;
                    for i in 0..3 {
                        point += Vec3::from_array(body[ids[i]].position) * weights[i];
                        normal += normals[ids[i]] * weights[i];
                    }
                    point + normal.try_normalize().unwrap_or(Vec3::Z) * 0.00008
                })
                .collect();
            for pair in points.windows(2) {
                let start = vertices.len();
                tube(
                    vertices,
                    indices,
                    pair[0],
                    pair[1],
                    0.00009,
                    [0.36, 0.17, 0.16, 1.],
                );
                // A wet mucocutaneous edge uses the tissue material, not atlas UV.
                for vertex in &mut vertices[start..] {
                    vertex.uv = [-1., 0.24];
                }
            }
        }
        self.append_lips(body, vertices, indices, head, pose);
        append_oral_anatomy_pose(vertices, indices, head, pose);
    }
    fn append_lips(
        &self,
        body: &[SceneVertex],
        vertices: &mut Vec<SceneVertex>,
        indices: &mut Vec<u32>,
        head: Mat4,
        pose: crate::female_face::FacePose,
    ) {
        if std::env::var("VOXY_FACE_DIAGNOSTIC_NO_LIP_REPLACEMENT").as_deref() == Ok("1") {
            return;
        }
        for &(ids, original) in &self.seams {
            let factor = |p: Vec3| {
                let smooth = |a: f32, b: f32, x: f32| {
                    let q = ((x - a) / (b - a)).clamp(0., 1.);
                    q * q * (3. - 2. * q)
                };
                crate::female_face::mouth_displacement(p, pose.jaw) * smooth(0.08, 0.12, p.z)
            };
            let source = ids.map(|i| body[i as usize]);
            let factors = original.map(factor);
            let mut tiles = vec![[Vec3::X, Vec3::Y, Vec3::Z]];
            for _ in 0..2 {
                tiles = tiles
                    .into_iter()
                    .flat_map(|[a, b, c]| {
                        let ab = (a + b) * 0.5;
                        let bc = (b + c) * 0.5;
                        let ca = (c + a) * 0.5;
                        [[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]]
                    })
                    .collect();
            }
            for weights in tiles {
                let rest =
                    weights.map(|w| original[0] * w.x + original[1] * w.y + original[2] * w.z);
                let inherited =
                    weights.map(|w| factors[0] * w.x + factors[1] * w.y + factors[2] * w.z);
                let mut posed = weights.map(|w| SceneVertex {
                    position: (Vec3::from_array(source[0].position) * w.x
                        + Vec3::from_array(source[1].position) * w.y
                        + Vec3::from_array(source[2].position) * w.z)
                        .to_array(),
                    uv: std::array::from_fn(|i| {
                        source[0].uv[i] * w.x + source[1].uv[i] * w.y + source[2].uv[i] * w.z
                    }),
                    color: std::array::from_fn(|i| {
                        source[0].color[i] * w.x
                            + source[1].color[i] * w.y
                            + source[2].color[i] * w.z
                    }),
                });
                for i in 0..3 {
                    let correction = head.transform_vector3(factor(rest[i]) - inherited[i]);
                    posed[i].position =
                        (Vec3::from_array(posed[i].position) + correction).to_array();
                }
                let inherited = rest.map(factor);
                let bind_normal = (rest[1] - rest[0])
                    .cross(rest[2] - rest[0])
                    .try_normalize()
                    .unwrap_or(Vec3::Z);
                let inner_lip =
                    rest.iter().map(|p| p.z).sum::<f32>() / 3. < 0.150 && bind_normal.z < 0.35;
                for upper in [true, false] {
                    let mut polygon = Vec::new();
                    for edge in 0..3 {
                        let next = (edge + 1) % 3;
                        let inside = if upper {
                            seam_distance(rest[edge]) >= 0.
                        } else {
                            seam_distance(rest[edge]) <= 0.
                        };
                        let next_inside = if upper {
                            seam_distance(rest[next]) >= 0.
                        } else {
                            seam_distance(rest[next]) <= 0.
                        };
                        if inside {
                            polygon.push(posed[edge]);
                        }
                        if inside != next_inside {
                            let t = seam_intersection(rest[edge], rest[next]);
                            let a = posed[edge];
                            let b = posed[next];
                            let bind = rest[edge].lerp(rest[next], t);
                            let existing = inherited[edge] * (1. - t) + inherited[next] * t;
                            let desired = if upper {
                                Vec3::ZERO
                            } else {
                                factor(Vec3::new(
                                    bind.x,
                                    crate::female_face::mouth_seam(bind.x) - 0.000_001,
                                    bind.z,
                                ))
                            };
                            let correction = head.transform_vector3(desired - existing);
                            polygon.push(SceneVertex {
                                position: (Vec3::from_array(a.position)
                                    .lerp(Vec3::from_array(b.position), t)
                                    + correction)
                                    .to_array(),
                                uv: std::array::from_fn(|i| a.uv[i] * (1. - t) + b.uv[i] * t),
                                color: std::array::from_fn(|i| {
                                    a.color[i] * (1. - t) + b.color[i] * t
                                }),
                            });
                        }
                    }
                    if inner_lip {
                        for vertex in &mut polygon {
                            vertex.uv = [-1., 0.36];
                            vertex.color = [0.35, 0.085, 0.10, 1.];
                        }
                    }
                    let base = u32::try_from(vertices.len()).expect("lip vertices");
                    if std::env::var("VOXY_FACE_DIAGNOSTIC_LIP_BIND").as_deref() == Ok("1")
                        && [119594_u32, 119588, 114385]
                            .into_iter()
                            .any(|id| id >= base && id < base + polygon.len() as u32)
                    {
                        eprintln!(
                            "LIP BIND base={base} upper={upper} inner={inner_lip} rest={rest:?} normal={bind_normal:?}"
                        );
                    }
                    vertices.extend_from_slice(&polygon);
                    for i in 1..polygon.len().saturating_sub(1) {
                        indices.extend([
                            base,
                            base + u32::try_from(i).unwrap(),
                            base + u32::try_from(i + 1).unwrap(),
                        ]);
                    }
                }
            }
        }
    }
}
/// Closest-point barycentrics, including edge and vertex Voronoi regions.
fn closest_triangle_weights(point: Vec3, triangle: [Vec3; 3]) -> Vec3 {
    let [a, b, c] = triangle;
    let ab = b - a;
    let ac = c - a;
    if ab.cross(ac).length_squared() < 1e-20 {
        let nearest = triangle
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                a.distance_squared(point)
                    .total_cmp(&b.distance_squared(point))
            })
            .unwrap()
            .0;
        return [Vec3::X, Vec3::Y, Vec3::Z][nearest];
    }
    let ap = point - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0. && d2 <= 0. {
        return Vec3::X;
    }
    let bp = point - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0. && d4 <= d3 {
        return Vec3::Y;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0. && d1 >= 0. && d3 <= 0. {
        let v = d1 / (d1 - d3);
        return Vec3::new(1. - v, v, 0.);
    }
    let cp = point - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0. && d5 <= d6 {
        return Vec3::Z;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0. && d2 >= 0. && d6 <= 0. {
        let w = d2 / (d2 - d6);
        return Vec3::new(1. - w, 0., w);
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0. && d4 - d3 >= 0. && d5 - d6 >= 0. {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return Vec3::new(0., 1. - w, w);
    }
    let denominator = va + vb + vc;
    if denominator.abs() < 1e-20 {
        return Vec3::X;
    }
    let v = vb / denominator;
    let w = vc / denominator;
    Vec3::new(1. - v - w, v, w)
}
fn seam_distance(point: Vec3) -> f32 {
    point.y - crate::female_face::mouth_seam(point.x)
}
fn seam_intersection(a: Vec3, b: Vec3) -> f32 {
    let mut low = 0.;
    let mut high = 1.;
    let positive = seam_distance(a) >= 0.;
    for _ in 0..24 {
        let middle = (low + high) * 0.5;
        if (seam_distance(a.lerp(b, middle)) >= 0.) == positive {
            low = middle;
        } else {
            high = middle;
        }
    }
    (low + high) * 0.5
}
fn is_seal(points: [Vec3; 3]) -> bool {
    let center = (points[0] + points[1] + points[2]) / 3.;
    let crosses = points.iter().any(|&p| seam_distance(p) < 0.)
        && points.iter().any(|&p| seam_distance(p) >= 0.);
    // Include the narrow adjoining lip band, so nonlinear jaw weights are
    // resolved inside its triangles instead of leaving a coarse inner shelf.
    let adjoining = seam_distance(center).abs() < 0.003 && center.z > 0.150 && center.z < 0.155;
    center.x.abs() < 0.032 && center.z > 0.132 && (crosses || adjoining)
}
/// Remove sealed lip triangles for replacement with two clipped lip surfaces.
/// The bind-pose replacement covers the same surface; source OBJ stays intact.
/// The topology is chosen once, so opening the jaw does not resize GPU buffers.
pub(crate) fn open_mouth_surface(vertices: &[SceneVertex], indices: &mut Vec<u32>) -> usize {
    let before = indices.len();
    let mut kept = Vec::with_capacity(before);
    for triangle in indices.chunks_exact(3) {
        let points: [Vec3; 3] =
            std::array::from_fn(|i| Vec3::from_array(vertices[triangle[i] as usize].position));
        if !is_seal(points) {
            kept.extend_from_slice(triangle);
        }
    }
    *indices = kept;
    (before - indices.len()) / 3
}
#[allow(clippy::many_single_char_names)] // Local tube basis and endpoint notation.
fn groom_noise(seed: usize) -> f32 {
    let mut bits = (seed as u32).wrapping_add(0x9e37_79b9);
    bits = (bits ^ (bits >> 16)).wrapping_mul(0x85eb_ca6b);
    bits = (bits ^ (bits >> 13)).wrapping_mul(0xc2b2_ae35);
    bits ^= bits >> 16;
    (bits >> 8) as f32 / 16_777_215.
}
fn lash_rotation(blink: f32) -> Quat {
    Quat::from_rotation_x(1.35 * blink.clamp(0., 1.))
}

// Bound the transverse curl by each strand's own length. The former fixed
// bow could reverse the root tangent on short corner lashes and form a hook.
fn lash_bend(direction: Vec3, lower: bool, seed: usize) -> Vec3 {
    let curl = 0.7 + 0.6 * groom_noise(seed + 1201);
    let lateral = 0.0005 * (groom_noise(seed + 1601) - 0.5);
    Vec3::new(
        lateral * if lower { 0.5 } else { 1. },
        -direction.y * (0.14 * curl),
        direction.z * (0.18 * curl),
    )
}

// The root follows the posed triangle. Near closure, constrain its orientation
// around a lid hinge to avoid following local fold inversions as a rigid turn.
// The 0.35 rad allowance is an artistic rig limit, not measured anatomy.
fn constrain_lash_rotation(surface: Quat, hinge: Quat, blink: f32) -> Quat {
    let angle = surface.angle_between(hinge);
    let limited = if angle > 0.35 {
        hinge.slerp(surface, 0.35 / angle)
    } else {
        surface
    };
    let t = ((blink - 0.5) / 0.5).clamp(0., 1.);
    surface.slerp(limited, t * t * (3. - 2. * t)).normalize()
}

fn skin_frame(triangle: [Vec3; 3]) -> Option<Quat> {
    let tangent = (triangle[1] - triangle[0]).try_normalize()?;
    let normal = (triangle[1] - triangle[0])
        .cross(triangle[2] - triangle[0])
        .try_normalize()?;
    let bitangent = normal.cross(tangent);
    Some(Quat::from_mat3(&Mat3::from_cols(tangent, bitangent, normal)).normalize())
}
/// Eight spans and a stable cross-section frame resolve the curled silhouette.
fn curved_lash(
    vertices: &mut Vec<SceneVertex>,
    indices: &mut Vec<u32>,
    root: Vec3,
    direction: Vec3,
    bend: Vec3,
    contact: Option<(&physics::hair::TriangleMesh, Vec3, &[[f64; 3]])>,
    radius: f32,
    color: [f32; 4],
) {
    let base = u32::try_from(vertices.len()).expect("lash vertex count");
    const SPANS: u32 = 16;
    const SIDES: u32 = 6;
    let mut points: Vec<_> = (0..=SPANS)
        .map(|row| {
            let t = row as f32 / SPANS as f32;
            root + direction * t + bend * (4. * t * (1. - t))
        })
        .collect();
    let mut corrected = false;
    if let Some((skin, forward, normals)) = contact {
        let trace = (root.x + 0.021336).abs() < 0.00005
            && std::env::var("VOXY_FACE_DIAGNOSTIC_LASH_CONTACT_TRACE").as_deref() == Ok("1");
        for pass in 0..3 {
            for row in 1..points.len() {
                let point = points[row];
                let t = row as f32 / SPANS as f32;
                let clearance = radius * (0.04 + 0.96 * (1. - t).powf(0.65)) + 0.00001;
                let query = point.to_array().map(f64::from);
                let nearest = if normals.is_empty() {
                    skin.closest_surface(query)
                } else {
                    skin.closest_surface_interpolated(query, normals)
                };
                if let Ok((surface, normal)) = nearest {
                    let surface = Vec3::from_array(surface.map(|v| v as f32));
                    let mut normal = Vec3::from_array(normal.map(|v| v as f32));
                    if normal.dot(forward) < 0. {
                        normal = -normal;
                    }
                    let delta = point - surface;
                    let gap = delta.dot(normal);
                    if trace {
                        eprintln!(
                            "LASH CONTACT TRACE pass={pass} row={row} query={point:?} surface={surface:?} normal={normal:?} distance={} gap={gap}",
                            delta.length()
                        );
                    }
                    if delta.length() < 0.002 && gap < clearance {
                        let correction = normal * (clearance - gap).min(0.0005);
                        // A contact bends a neighborhood instead of one isolated ring.
                        // The root remains fixed, including for contacts near it.
                        for (neighbor, position) in points.iter_mut().enumerate().skip(1) {
                            let distance = neighbor as f32 - row as f32;
                            let weight = (-distance * distance / 8.).exp()
                                * (neighbor as f32 / row as f32).min(1.);
                            *position += correction * weight;
                        }
                        corrected = true;
                    }
                }
            }
        }
    }
    if let Some((skin, forward, _)) = contact {
        for _ in 0..6 {
            for span in 0..SPANS as usize {
                let t = span as f32 / SPANS as f32;
                let clearance = radius * (0.04 + 0.96 * (1. - t).powf(0.65)) + 0.00001;
                if let Ok(Some((fraction, correction))) = skin.capsule_contact(
                    points[span].to_array().map(f64::from),
                    points[span + 1].to_array().map(f64::from),
                    f64::from(clearance),
                    forward.to_array().map(f64::from),
                ) {
                    let row = span as f32 + fraction as f32;
                    let correction = Vec3::from_array(correction.map(|v| v as f32));
                    for (neighbor, point) in points.iter_mut().enumerate().skip(1) {
                        let d = neighbor as f32 - row;
                        *point += correction
                            * (-d * d / 8.).exp()
                            * (neighbor as f32 / row.max(0.5)).min(1.);
                    }
                    corrected = true;
                }
            }
        }
    }
    let initial_tangent = (direction + bend * 4.).normalize();
    let reference = if initial_tangent.y.abs() < 0.9 {
        Vec3::Y
    } else {
        Vec3::X
    };
    let initial_u = initial_tangent.cross(reference).normalize();
    for row in 0..=SPANS {
        let t = row as f32 / SPANS as f32;
        let point = points[row as usize];
        let tangent = if corrected {
            (points[(row as usize + 1).min(SPANS as usize)]
                - points[row.saturating_sub(1) as usize])
                .try_normalize()
                .unwrap_or(initial_tangent)
        } else {
            (direction + bend * (4. - 8. * t)).normalize()
        };
        let u = (initial_u - tangent * initial_u.dot(tangent)).normalize();
        let v = tangent.cross(u);
        let r = radius * (0.04 + 0.96 * (1. - t).powf(0.65));
        for segment in 0..SIDES {
            let angle = segment as f32 * std::f32::consts::TAU / SIDES as f32;
            vertices.push(SceneVertex {
                position: (point + (u * angle.cos() + v * angle.sin()) * r).to_array(),
                uv: [-0.25, 0.45],
                color,
            });
        }
    }
    for row in 0..SPANS {
        for segment in 0..SIDES {
            let a = base + row * SIDES + segment;
            let b = base + row * SIDES + (segment + 1) % SIDES;
            indices.extend([a, b, a + SIDES, b, b + SIDES, a + SIDES]);
        }
    }
}
fn tube(
    vertices: &mut Vec<SceneVertex>,
    indices: &mut Vec<u32>,
    a: Vec3,
    b: Vec3,
    radius: f32,
    color: [f32; 4],
) {
    let axis = (b - a).try_normalize().unwrap_or(Vec3::Z);
    let reference = if axis.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
    let u = axis.cross(reference).normalize();
    let v = axis.cross(u);
    let base = u32::try_from(vertices.len()).expect("face vertex count");
    for row in 0..2 {
        for segment in 0..4 {
            let angle = segment as f32 * std::f32::consts::FRAC_PI_2;
            let point = if row == 0 { a } else { b };
            let r = if row == 0 { radius } else { radius * 0.2 };
            vertices.push(SceneVertex {
                position: (point + (u * angle.cos() + v * angle.sin()) * r).to_array(),
                uv: [0.; 2],
                color,
            });
        }
    }
    for segment in 0..4 {
        let next = (segment + 1) % 4;
        indices.extend([
            base + segment,
            base + next,
            base + 4 + segment,
            base + next,
            base + 4 + next,
            base + 4 + segment,
        ]);
    }
}
/// Two complete dental arches, gingiva, tongue, palate and recessed pharynx.
/// Lower structures share the same jaw displacement as the lower lip.
#[cfg(test)]
fn append_oral_anatomy(
    vertices: &mut Vec<SceneVertex>,
    indices: &mut Vec<u32>,
    head: Mat4,
    jaw: f32,
) {
    append_oral_anatomy_pose(
        vertices,
        indices,
        head,
        crate::female_face::FacePose {
            jaw,
            ..Default::default()
        },
    );
}
fn append_oral_anatomy_pose(
    vertices: &mut Vec<SceneVertex>,
    indices: &mut Vec<u32>,
    head: Mat4,
    pose: crate::female_face::FacePose,
) {
    let jaw = pose.jaw;
    let omit_teeth = std::env::var("VOXY_FACE_DIAGNOSTIC_NO_TEETH").as_deref() == Ok("1");
    let mut crowns = Vec::new();
    for lower in [false, true] {
        let drop = if lower { 0.010 * jaw } else { 0. };
        let gum_y = if lower { 0.637 } else { 0.649 } - drop;
        if std::env::var("VOXY_FACE_DIAGNOSTIC_NO_GINGIVA").as_deref() != Ok("1") {
            append_gingival_arch(vertices, indices, head, gum_y, lower);
        }
        // Incisors, canines, premolars and molars, mirrored on each side.
        for side in [-1., 1.] {
            for tooth in 0..8 {
                let angle = (tooth as f32 + 0.5) * std::f32::consts::FRAC_PI_2 / 8.;
                let x = side * 0.025 * angle.sin();
                let z = 0.105 + 0.043 * angle.cos();
                let (width, height, depth) = match tooth {
                    0 | 1 => (0.0023, 0.0034, 0.0016),
                    2 => (0.0021, 0.0035, 0.0022),
                    3 | 4 => (0.0025, 0.0028, 0.0028),
                    _ => (0.0031, 0.0026, 0.0033),
                };
                let center_y = gum_y + if lower { height * 0.8 } else { -height * 0.8 };
                let tooth_frame = head
                    * Mat4::from_translation(Vec3::new(x, center_y, z))
                    * Mat4::from_rotation_y(side * angle);
                crowns.push(CrownEnvelope {
                    inverse: tooth_frame.inverse(),
                    radii: Vec3::new(width, height, depth) + Vec3::splat(0.0002),
                    tooth,
                    lower,
                });
                if !omit_teeth {
                    tooth_crown(
                        vertices,
                        indices,
                        tooth_frame,
                        Vec3::new(width, height, depth),
                        tooth,
                        lower,
                    );
                }
            }
        }
    }
    let start = vertices.len();
    append_tongue(vertices, indices, head, jaw);
    let inverse = head.inverse();
    let aperture = jaw.clamp(0., 1.);
    for vertex in &mut vertices[start..] {
        let mut point = inverse.transform_point3(Vec3::from_array(vertex.position));
        let anterior = ((point.z - 0.103) / 0.036).clamp(0., 1.);
        let weight = anterior * anterior * (3. - 2. * anterior);
        point.y += 0.003 * pose.tongue_lift.clamp(-1., 1.) * aperture * weight;
        point.z += 0.010 * pose.tongue_forward.clamp(0., 1.) * aperture * weight;
        let mut world = head.transform_point3(point);
        if aperture > 0. {
            let retreat = head.transform_vector3(-Vec3::Z * 0.015);
            for crown in &crowns {
                if crown.contains(world) {
                    let mut lo = 0.;
                    let mut hi = 1.;
                    for _ in 0..24 {
                        let t = (lo + hi) * 0.5;
                        if crown.contains(world + retreat * t) {
                            lo = t;
                        } else {
                            hi = t;
                        }
                    }
                    world += retreat * hi;
                }
            }
        }
        vertex.position = world.to_array();
    }
    if std::env::var("VOXY_FACE_DIAGNOSTIC_NO_PALATE").as_deref() != Ok("1") {
        ellipsoid(
            vertices,
            indices,
            head,
            Vec3::new(0., 0.651, 0.118),
            Vec3::new(0.020, 0.0028, 0.019),
            [0.30, 0.09, 0.105, 1.],
        );
    }
    // Buccal lining surrounds the teeth and joins the deeper oral cavity.
    // An open front preserves the lip aperture; side walls cover the head shell
    // that would otherwise be seen as beige skin behind the dental arches.
    if std::env::var("VOXY_FACE_DIAGNOSTIC_NO_BUCCAL").as_deref() != Ok("1") {
        let lining_base = u32::try_from(vertices.len()).expect("buccal lining vertex count");
        for ring in 0..=8 {
            let t = ring as f32 / 8.;
            for segment in 0..24 {
                let angle = segment as f32 * std::f32::consts::TAU / 24.;
                let x = (0.026 - 0.008 * t) * angle.cos();
                let front_x = 0.026 * angle.cos();
                let seam = crate::female_face::mouth_seam(front_x);
                let front_z = 0.148 - 5. * front_x * front_x;
                let lower_weight =
                    crate::female_face::jaw_weight(Vec3::new(front_x, seam - 0.000001, front_z));
                let opening = lower_weight * (-angle.sin()).max(0.) * jaw;
                let front_y = seam + 0.001 * angle.sin() - 0.010 * opening;
                let back_y = 0.640 - 0.005 * jaw + (0.009 + 0.003 * jaw) * angle.sin();
                let point = Vec3::new(
                    x,
                    front_y * (1. - t) + back_y * t,
                    0.148 - 0.040 * t - (5. + 15. * t) * x * x - 0.002 * opening * (1. - t),
                );
                vertices.push(SceneVertex {
                    position: head.transform_point3(point).to_array(),
                    uv: [-1., 0.42],
                    color: [0.23, 0.060, 0.078, 1.],
                });
            }
        }
        for ring in 0..8 {
            for segment in 0..24 {
                let a = lining_base + ring * 24 + segment;
                let b = lining_base + ring * 24 + (segment + 1) % 24;
                indices.extend([a, b, a + 24, b, b + 24, a + 24]);
            }
        }
    }
    // A narrowing tunnel avoids a flat black plate immediately behind the teeth.
    let base = u32::try_from(vertices.len()).expect("pharynx vertex count");
    for ring in 0..=8 {
        let t = ring as f32 / 8.;
        for segment in 0..24 {
            let angle = segment as f32 * std::f32::consts::TAU / 24.;
            let x = (0.018 - 0.011 * t) * angle.cos();
            let point = Vec3::new(
                x,
                0.640 - 0.005 * jaw
                    + (0.009 + 0.003 * jaw - (0.005 + 0.003 * jaw) * t) * angle.sin(),
                0.108 - 0.023 * t - 20. * x * x * (1. - t),
            );
            let light = 1. - 0.85 * t;
            vertices.push(SceneVertex {
                position: head.transform_point3(point).to_array(),
                uv: [-1., 0.55],
                color: [0.12 * light, 0.026 * light, 0.037 * light, 1.],
            });
        }
    }
    for ring in 0..8 {
        for segment in 0..24 {
            let a = base + ring * 24 + segment;
            let b = base + ring * 24 + (segment + 1) % 24;
            indices.extend([a, b, a + 24, b, b + 24, a + 24]);
        }
    }
    ellipsoid(
        vertices,
        indices,
        head,
        Vec3::new(0., 0.640 - 0.005 * jaw, 0.084),
        Vec3::new(0.007, 0.004, 0.002),
        [0.008, 0.002, 0.003, 1.],
    );
    ellipsoid(
        vertices,
        indices,
        head,
        Vec3::new(0., 0.645, 0.106),
        Vec3::new(0.0018, 0.0037, 0.002),
        [0.33, 0.095, 0.11, 1.],
    );
}
const CANINE_TAPER: f32 = 0.20;
// One connected arch replaces overlapping independent collars. Scalloping
// follows the dental spacing; this remains a procedural approximation.
fn append_gingival_arch(
    vertices: &mut Vec<SceneVertex>,
    indices: &mut Vec<u32>,
    head: Mat4,
    gum_y: f32,
    lower: bool,
) {
    const ROWS: u32 = 128;
    const SIDES: u32 = 16;
    let base = vertices.len() as u32;
    for row in 0..=ROWS {
        let angle = -std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * row as f32 / ROWS as f32;
        let radial = Vec3::new(angle.sin() / 0.025, 0., angle.cos() / 0.043).normalize();
        let center = Vec3::new(0.025 * angle.sin(), gum_y, 0.104 + 0.043 * angle.cos());
        let width = 0.0025 + 0.0013 * angle.sin().abs();
        let scallop = 0.0006 * (32. * angle).cos() * if lower { 1. } else { -1. };
        for side in 0..SIDES {
            let phi = std::f32::consts::TAU * side as f32 / SIDES as f32;
            let toward_teeth = phi.sin() * if lower { 1. } else { -1. };
            let edge_weight = toward_teeth.max(0.).powi(2);
            let local = center
                + radial * (width * phi.cos())
                + Vec3::Y * (0.0024 * phi.sin() + scallop * edge_weight);
            vertices.push(SceneVertex {
                position: head.transform_point3(local).to_array(),
                uv: [-1., 0.36],
                color: [0.38, 0.105, 0.115, 1.],
            });
        }
    }
    for row in 0..ROWS {
        for side in 0..SIDES {
            let a = base + row * SIDES + side;
            let b = base + row * SIDES + (side + 1) % SIDES;
            indices.extend([a, a + SIDES, b, b, a + SIDES, b + SIDES]);
        }
    }
    // Cap both posterior ends without duplicate seam vertices.
    for row in [0, ROWS] {
        let center = vertices[base as usize + row as usize * SIDES as usize
            ..base as usize + (row as usize + 1) * SIDES as usize]
            .iter()
            .map(|v| Vec3::from_array(v.position))
            .sum::<Vec3>()
            / SIDES as f32;
        let cap = vertices.len() as u32;
        vertices.push(SceneVertex {
            position: center.to_array(),
            uv: [-1., 0.36],
            color: [0.38, 0.105, 0.115, 1.],
        });
        for side in 0..SIDES {
            let a = base + row * SIDES + side;
            let b = base + row * SIDES + (side + 1) % SIDES;
            if row == 0 {
                indices.extend([cap, a, b]);
            } else {
                indices.extend([cap, b, a]);
            }
        }
    }
}
struct CrownEnvelope {
    inverse: Mat4,
    radii: Vec3,
    tooth: usize,
    lower: bool,
}
impl CrownEnvelope {
    fn contains(&self, point: Vec3) -> bool {
        let mut p = self.inverse.transform_point3(point) / self.radii;
        let exponent = crown_exponent(self.tooth);
        if self.tooth >= 3 {
            let power = 2. / exponent;
            let remaining = 1. - p.x.abs().powf(power) - p.z.abs().powf(power);
            if remaining <= 0. {
                return false;
            }
            let height = remaining.powf(1. / power);
            let cutting = if self.lower { p.y } else { -p.y };
            let surface = height - crown_fissure_depth(p.x, p.z, height, self.tooth) / self.radii.y;
            return cutting > -height && cutting < surface;
        }
        if self.tooth == 2 {
            let cutting = if self.lower { p.y } else { -p.y };
            p.x /= 1. - CANINE_TAPER * cutting.clamp(0., 1.);
        }
        p.abs().map(|v| v.powf(2. / exponent)).element_sum() < 1.
    }
}
fn crown_fissure_depth(x: f32, z: f32, cutting_side: f32, tooth: usize) -> f32 {
    let longitudinal = (-(x / 0.22).powi(2)).exp();
    let crossing = if tooth >= 5 {
        (-(z / 0.24).powi(2)).exp()
    } else {
        0.
    };
    let exposure = ((cutting_side - 0.45) / 0.55).clamp(0., 1.);
    0.0005 * longitudinal.max(crossing) * exposure * exposure
}
fn crown_exponent(tooth: usize) -> f32 {
    if tooth == 2 {
        0.55
    } else if tooth < 2 {
        0.38
    } else {
        0.5
    }
}
/// One continuous dorsum avoids overlapping lobes and their internal seam.
fn append_tongue(vertices: &mut Vec<SceneVertex>, indices: &mut Vec<u32>, head: Mat4, jaw: f32) {
    let start = vertices.len();
    let center = Vec3::new(0., 0.642 - 0.008 * jaw, 0.121);
    ellipsoid(
        vertices,
        indices,
        Mat4::IDENTITY,
        center,
        Vec3::new(0.016, 0.0035, 0.018),
        [0.40, 0.13, 0.145, 1.],
    );
    for vertex in &mut vertices[start..] {
        let mut local = Vec3::from_array(vertex.position) - center;
        let forward = (local.z / 0.018).max(0.);
        local.x *= 1. - 0.18 * forward * forward;
        let dorsal = (local.y / 0.0035).max(0.);
        let groove =
            (-(local.x / 0.0022).powi(2)).exp() * (1. - (local.z / 0.018).powi(2)).max(0.) * dorsal;
        let dome = (1. - (local.x / 0.016).powi(2)).max(0.)
            * (1. - (local.z / 0.018).powi(2)).max(0.)
            * dorsal;
        local.y += 0.0015 * jaw.clamp(0., 1.) * dome;
        local.y -= 0.00055 * groove;
        vertex.position = head.transform_point3(center + local).to_array();
        if dorsal > 0. {
            // Reserved tongue material range, with bind-space X/Z coordinates.
            vertex.uv = [
                -1.4 + 0.2 * (0.5 + 0.5 * local.x / 0.016),
                0.5 + 0.5 * local.z / 0.018,
            ];
        }
        let warmth = 0.035 * dorsal + 0.025 * forward;
        vertex.color = [0.40 + warmth, 0.13 + warmth * 0.7, 0.145 + warmth * 0.6, 1.];
    }
}
/// Rounded rectangular incisors retain a cutting edge; posterior teeth have
/// broader occlusal surfaces. Canines retain a tapered crown.
fn tooth_crown(
    vertices: &mut Vec<SceneVertex>,
    indices: &mut Vec<u32>,
    frame: Mat4,
    radii: Vec3,
    tooth: usize,
    lower: bool,
) {
    let start = vertices.len();
    let resolution = if tooth >= 3 { [32, 48] } else { [16, 32] };
    ellipsoid_sampled(
        vertices,
        indices,
        frame,
        Vec3::ZERO,
        radii,
        [0.73, 0.70, 0.61, 1.],
        resolution,
        tooth >= 3,
    );
    let inverse = frame.inverse();
    let exponent = crown_exponent(tooth);
    for (sample, vertex) in vertices[start..].iter_mut().enumerate() {
        let unit = inverse.transform_point3(Vec3::from_array(vertex.position)) / radii;
        let mut shaped = unit.map(|v| {
            if v.abs() < 0.00001 {
                0.
            } else {
                v.signum() * v.abs().powf(exponent)
            }
        });
        let cutting_side = if lower { shaped.y } else { -shaped.y };
        if tooth == 2 {
            shaped.x *= 1. - CANINE_TAPER * cutting_side.max(0.);
        }
        if tooth >= 3 {
            // Carve the occlusal fissures inward: a longitudinal groove leaves
            // two premolar cusps; molars also receive a crossing groove.
            let depth = crown_fissure_depth(shaped.x, shaped.z, cutting_side, tooth);
            shaped.y -= if lower {
                depth / radii.y
            } else {
                -depth / radii.y
            };
        }
        // Cervical enamel is warmer; the cutting edge has less dentin tint.
        let row = sample / (resolution[1] as usize + 1);
        let t = row as f32 / resolution[0] as f32;
        let latitude = if tooth >= 3 {
            0.5 * (1. - (t * std::f32::consts::PI).cos())
        } else {
            t
        };
        let local_y = (latitude * std::f32::consts::PI).cos();
        let canonical_y = local_y.signum() * local_y.abs().powf(exponent);
        let canonical_cutting = if lower { canonical_y } else { -canonical_y };
        let edge = ((canonical_cutting + 1.) * 0.5).clamp(0., 1.);
        let enamel = Vec3::new(0.66, 0.605, 0.49).lerp(Vec3::new(0.77, 0.765, 0.70), edge);
        vertex.color = [enamel.x, enamel.y, enamel.z, 1.];
        // Small labial development lobes give incisors a gently undulating
        // face. The 80 um displacement stays inside the crown contact margin.
        // This is an artistic crown model, not patient-specific anatomy.
        if tooth < 2 && shaped.z > 0. {
            let lobes = [-0.55_f32, 0., 0.55]
                .into_iter()
                .map(|center| (-((shaped.x - center) / 0.22).powi(2)).exp())
                .sum::<f32>();
            let taper = (1. - shaped.y * shaped.y).max(0.) * shaped.z;
            shaped.z += 0.00008 / radii.z * lobes * taper;
        }
        vertex.uv[1] = 0.24 + 0.08 * (1. - edge);
        vertex.position = frame.transform_point3(shaped * radii).to_array();
    }
}
fn ellipsoid(
    vertices: &mut Vec<SceneVertex>,
    indices: &mut Vec<u32>,
    head: Mat4,
    center: Vec3,
    radii: Vec3,
    color: [f32; 4],
) {
    ellipsoid_sampled(
        vertices,
        indices,
        head,
        center,
        radii,
        color,
        [16, 32],
        false,
    );
}
#[allow(clippy::too_many_arguments)]
fn ellipsoid_sampled(
    vertices: &mut Vec<SceneVertex>,
    indices: &mut Vec<u32>,
    head: Mat4,
    center: Vec3,
    radii: Vec3,
    color: [f32; 4],
    resolution: [u32; 2],
    concentrated_poles: bool,
) {
    let base = u32::try_from(vertices.len()).expect("oral vertex count");
    let [rows, columns] = resolution;
    // Latitude bands omit degenerate pole triangles.
    for row in 0..=rows {
        for column in 0..=columns {
            let t = row as f32 / rows as f32;
            // Superellipsoid crowns stretch the first latitude ring away from
            // the pole. Concentration restores samples inside narrow fissures.
            let latitude = if concentrated_poles {
                0.5 * (1. - (t * std::f32::consts::PI).cos())
            } else {
                t
            };
            let phi = latitude * std::f32::consts::PI;
            let theta = column as f32 * std::f32::consts::TAU / columns as f32;
            let n = Vec3::new(phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin());
            vertices.push(SceneVertex {
                position: head.transform_point3(center + n * radii).to_array(),
                // Negative U tags a wet oral surface; V stores roughness.
                uv: [
                    -1.,
                    if color[0] > 0.6 {
                        0.27
                    } else if color[0] > 0.2 {
                        0.36
                    } else {
                        0.85
                    },
                ],
                color,
            });
        }
    }
    for row in 0..rows {
        for column in 0..columns {
            let a = base + row * (columns + 1) + column;
            let b = a + columns + 1;
            if row != 0 {
                indices.extend([a, a + 1, b]);
            }
            if row != rows - 1 {
                indices.extend([a + 1, b + 1, b]);
            }
        }
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn lash_contact_preserves_root_and_clears_a_plane_with_its_radius() {
        let skin = physics::hair::TriangleMesh::new(
            &[
                [-0.1, -0.1, 0.],
                [0.1, -0.1, 0.],
                [0.1, 0.1, 0.],
                [-0.1, 0.1, 0.],
            ],
            &[[0, 1, 2], [0, 2, 3]],
        )
        .unwrap();
        let root = Vec3::new(0., 0., 0.0002);
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        curved_lash(
            &mut vertices,
            &mut indices,
            root,
            Vec3::new(0., 0.004, -0.0004),
            Vec3::new(0., 0., 0.0001),
            Some((&skin, Vec3::Z, &[])),
            0.000035,
            [1.; 4],
        );
        let center = vertices[..6]
            .iter()
            .map(|v| Vec3::from_array(v.position))
            .sum::<Vec3>()
            / 6.;
        assert!(center.distance(root) < 1e-8);
        assert!(
            vertices
                .iter()
                .all(|v| v.position.iter().all(|x| x.is_finite()) && v.position[2] >= 0.0000099)
        );
        voxy_render::SceneMesh::new(vertices, indices).unwrap();
    }
    #[test]
    fn gingival_arch_is_closed_connected_and_faces_outward() {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        append_gingival_arch(&mut vertices, &mut indices, Mat4::IDENTITY, 0.649, false);
        let mut edges = std::collections::HashMap::<(u32, u32), (usize, i32)>::new();
        for triangle in indices.chunks_exact(3) {
            let p = triangle
                .iter()
                .map(|&i| Vec3::from_array(vertices[i as usize].position))
                .collect::<Vec<_>>();
            assert!((p[1] - p[0]).cross(p[2] - p[0]).length_squared() > 1e-18);
            for k in 0..3 {
                let (a, b) = (triangle[k], triangle[(k + 1) % 3]);
                let entry = edges.entry((a.min(b), a.max(b))).or_default();
                entry.0 += 1;
                entry.1 += if a < b { 1 } else { -1 };
            }
        }
        assert!(
            edges
                .values()
                .all(|&(count, orientation)| count == 2 && orientation == 0)
        );
        let mut neighbours = vec![Vec::new(); vertices.len()];
        for &(a, b) in edges.keys() {
            neighbours[a as usize].push(b as usize);
            neighbours[b as usize].push(a as usize);
        }
        let mut visited = vec![false; vertices.len()];
        let mut pending = vec![0];
        while let Some(i) = pending.pop() {
            if visited[i] {
                continue;
            }
            visited[i] = true;
            pending.extend(neighbours[i].iter().copied());
        }
        assert!(visited.iter().all(|&v| v));
        let triangle = &indices[64 * 16 * 6..64 * 16 * 6 + 3];
        let p: Vec<_> = triangle
            .iter()
            .map(|&i| Vec3::from_array(vertices[i as usize].position))
            .collect();
        assert!((p[1] - p[0]).cross(p[2] - p[0]).z > 0.);
        assert_eq!(
            vertices.len() as isize - edges.len() as isize + (indices.len() / 3) as isize,
            2
        );
        if let Some(path) = std::env::var_os("VOXY_GUM_MESH_CAPTURE") {
            let positions: Vec<_> = vertices.iter().map(|v| v.position).collect();
            std::fs::write(path, serde_json::to_vec(&(positions, &indices)).unwrap()).unwrap();
        }
        SceneMesh::new(vertices, indices).unwrap();
    }
    #[test]
    fn crown_contact_excludes_fissure_space_but_keeps_cusps_and_cervical_face() {
        let frame =
            Mat4::from_translation(Vec3::new(0.01, 0.64, 0.13)) * Mat4::from_rotation_y(0.4);
        for tooth in [3, 5] {
            for lower in [false, true] {
                let crown = CrownEnvelope {
                    inverse: frame.inverse(),
                    radii: Vec3::new(0.0031, 0.0026, 0.0033),
                    tooth,
                    lower,
                };
                let sign = if lower { 1. } else { -1. };
                let contains = |p| crown.contains(frame.transform_point3(p));
                assert!(!contains(Vec3::new(0., sign * 0.0024, 0.)));
                assert!(contains(Vec3::new(0., sign * 0.0020, 0.)));
                assert!(contains(Vec3::new(0.0012, sign * 0.0024, 0.0012)));
                assert!(contains(Vec3::new(0., -sign * 0.0024, 0.)));
                assert!(!contains(Vec3::new(0.004, 0., 0.)));
            }
        }
    }
    #[test]
    fn actual_lash_curls_never_reverse_the_open_lid_root_tangent() {
        let asset = ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            ObjLimits::default(),
        )
        .unwrap();
        let features = FaceFeatures::new(asset.mesh.vertices(), asset.mesh.indices());
        let mut checked = 0;
        for (index, strand) in features.strands.iter().enumerate() {
            if !matches!(strand.kind, Kind::Lash | Kind::LowerLash) {
                continue;
            }
            let bend = lash_bend(
                strand.direction,
                matches!(strand.kind, Kind::LowerLash),
                index,
            );
            for step in 0..=32 {
                let t = step as f32 / 32.;
                let tangent = strand.direction + bend * (4. - 8. * t);
                assert!(
                    tangent.y * strand.direction.y > 0.,
                    "lash {index} hooks back into its lid at {t}"
                );
                assert!(
                    tangent.z > 0.,
                    "lash {index} doubles back toward the eye at {t}"
                );
            }
            checked += 1;
        }
        assert_eq!(checked, 144);
    }
    #[test]
    fn actual_closed_lash_contact_does_not_form_sharp_segment_reversals() {
        let mut preview = crate::face_preview::FacePreview::new().unwrap();
        for closure in [0., 0.5, 1.] {
            let mesh = preview
                .sample_blink(0., Vec3::new(0.033, 0.714, 0.20), closure)
                .unwrap();
            let lashes: Vec<_> = mesh
                .vertices()
                .iter()
                .filter(|v| v.uv == [-0.25, 0.45])
                .collect();
            assert_eq!(lashes.len(), 144 * 102);
            for (strand, vertices) in lashes.chunks_exact(102).enumerate() {
                let centers: Vec<_> = vertices
                    .chunks_exact(6)
                    .map(|ring| {
                        ring.iter()
                            .map(|v| Vec3::from_array(v.position))
                            .sum::<Vec3>()
                            / 6.
                    })
                    .collect();
                for span in 0..centers.len() - 2 {
                    let a = (centers[span + 1] - centers[span]).normalize();
                    let b = (centers[span + 2] - centers[span + 1]).normalize();
                    assert!(
                        a.dot(b) > std::f32::consts::FRAC_1_SQRT_2,
                        "lash {strand} at closure {closure}, span {span} bends over 45 degrees"
                    );
                }
            }
        }
    }
    #[test]
    fn actual_upper_lash_tips_point_down_at_full_closure() {
        let asset = ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            ObjLimits::default(),
        )
        .unwrap();
        let features = FaceFeatures::new(asset.mesh.vertices(), asset.mesh.indices());
        let contour =
            crate::female_face::LidContour::new(asset.mesh.vertices(), asset.mesh.indices());
        let mut posed = asset.mesh.vertices().to_vec();
        let count = posed.len();
        crate::female_face::deform_with_contour(
            &mut posed,
            count,
            crate::female_face::FacePose {
                blink: 1.,
                ..Default::default()
            },
            Some(&contour),
        );
        for strand in features
            .strands
            .iter()
            .filter(|s| matches!(s.kind, Kind::Lash))
        {
            let frame =
                skin_frame(strand.roots.map(|i| Vec3::from_array(posed[i].position))).unwrap();
            let rotation = constrain_lash_rotation(
                frame * strand.bind_frame.conjugate(),
                lash_rotation(1.),
                1.,
            );
            assert!(
                (rotation * strand.direction).y < 0.,
                "upper lash points up at full closure"
            );
        }
    }
    #[test]
    fn closed_lash_rotation_limits_fold_twist_and_preserves_head_motion() {
        let head = Quat::from_rotation_y(0.7);
        let hinge = head * lash_rotation(1.);
        let surface = head * Quat::from_rotation_x(-0.9);
        let closed = constrain_lash_rotation(surface, hinge, 1.);
        assert!(closed.angle_between(hinge) <= 0.35001);
        assert!(constrain_lash_rotation(surface, hinge, 0.).angle_between(surface) < 0.001);
        let local = constrain_lash_rotation(head.conjugate() * surface, lash_rotation(1.), 1.);
        assert!((head * local).angle_between(closed) < 0.001);
    }
    #[test]
    fn lash_attachment_frame_tracks_rigid_motion_and_rejects_collapsed_skin() {
        let triangle = [Vec3::ZERO, Vec3::X * 0.002, Vec3::Y * 0.001];
        let bind = skin_frame(triangle).unwrap();
        let motion = Quat::from_rotation_y(0.4) * Quat::from_rotation_x(0.8);
        let posed = triangle.map(|p| motion * p + Vec3::new(0.1, 0.7, 0.13));
        let transport = skin_frame(posed).unwrap() * bind.conjugate();
        let direction = Vec3::new(0.001, 0.004, 0.003);
        assert!((transport * direction - motion * direction).length() < 1e-7);
        assert!(((transport * direction).length() - direction.length()).abs() < 1e-7);
        assert!(skin_frame([Vec3::ZERO; 3]).is_none());
    }
    #[test]
    fn blink_rotates_lash_shape_without_shortening_it() {
        let direction = Vec3::new(0.001, 0.004, 0.003);
        let bend = Vec3::new(0., -0.0018, 0.0012);
        for closure in [0., 0.25, 0.5, 0.75, 1.] {
            let rotation = lash_rotation(closure);
            for t in [0.25, 0.5, 0.75, 1.] {
                let before = direction * t + bend * (4. * t * (1. - t));
                let after = rotation * direction * t + rotation * bend * (4. * t * (1. - t));
                assert!((before.length() - after.length()).abs() < 1e-7);
            }
        }
        assert!((lash_rotation(1.) * direction).y < 0.);
    }
    use super::*;
    use voxy_render::{ObjAsset, ObjLimits, SceneMesh};
    #[test]
    fn rendered_follicle_follows_animated_triangle_without_vertex_snapping() {
        let asset = ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            ObjLimits::default(),
        )
        .unwrap();
        let features = FaceFeatures::new(asset.mesh.vertices(), asset.mesh.indices());
        let strand = features.strands[0];
        let normals = vec![Vec3::Z; asset.mesh.vertices().len()];
        let render = |body: &[SceneVertex]| {
            let mut vertices = Vec::new();
            let mut indices = Vec::new();
            features.append(
                body,
                &normals,
                &mut vertices,
                &mut indices,
                Mat4::IDENTITY,
                Default::default(),
            );
            vertices[..4]
                .iter()
                .map(|v| Vec3::from_array(v.position))
                .sum::<Vec3>()
                / 4.
        };
        let rest = render(asset.mesh.vertices());
        let mut body = asset.mesh.vertices().to_vec();
        for (component, index) in strand.roots.iter().enumerate() {
            body[*index].position[1] += 0.002 * (component as f32 + 1.);
        }
        let moved = render(&body);
        let expected = 0.002 * (strand.weights.x + 2. * strand.weights.y + 3. * strand.weights.z);
        assert!((moved.y - rest.y - expected).abs() < 0.0000002);
    }
    #[test]
    fn follicle_projection_handles_surface_edges_and_degenerate_triangles() {
        let triangle = [Vec3::ZERO, Vec3::X, Vec3::Y];
        assert!(
            closest_triangle_weights(Vec3::new(0.2, 0.3, 1.), triangle)
                .distance(Vec3::new(0.5, 0.2, 0.3))
                < 1e-6
        );
        for point in [Vec3::new(2., 2., 1.), Vec3::new(-1., 0.2, 0.), Vec3::ZERO] {
            let weights = closest_triangle_weights(point, triangle);
            assert!(
                weights.is_finite()
                    && weights.min_element() >= 0.
                    && (weights.element_sum() - 1.).abs() < 1e-6
            );
        }
        assert!(closest_triangle_weights(Vec3::Z, [Vec3::ZERO; 3]).is_finite());
    }
    #[test]
    fn actual_follicles_have_distributed_surface_anchors() {
        let asset = ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            ObjLimits::default(),
        )
        .unwrap();
        let features = FaceFeatures::new(asset.mesh.vertices(), asset.mesh.indices());
        assert!(features.strands.iter().all(|s| s.weights.is_finite()
            && s.weights.min_element() >= -1e-6
            && (s.weights.element_sum() - 1.).abs() < 1e-5));
        let interior = features
            .strands
            .iter()
            .filter(|s| s.weights.min_element() > 0.01)
            .count();
        assert!(
            interior > 40,
            "only {interior} follicles projected inside faces"
        );
    }
    #[test]
    fn curved_seam_intersection_is_shared_by_both_lip_boundaries() {
        let a = Vec3::new(0.005, 0.648, 0.15);
        let b = Vec3::new(0.007, 0.643, 0.153);
        let t = seam_intersection(a, b);
        assert!(t > 0. && t < 1.);
        let forward = a.lerp(b, t);
        let reverse = b.lerp(a, seam_intersection(b, a));
        assert!(forward.distance(reverse) < 0.0000002);
        assert!(seam_distance(forward).abs() < 0.0000001);
        assert!(crate::female_face::mouth_seam(0.007) > crate::female_face::mouth_seam(0.026));
    }
    #[test]
    fn incisor_has_broad_cutting_edge_and_enamel_material() {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        tooth_crown(
            &mut vertices,
            &mut indices,
            Mat4::IDENTITY,
            Vec3::new(0.0023, 0.0034, 0.0016),
            0,
            false,
        );
        let edge: Vec<_> = vertices
            .iter()
            .filter(|v| v.position[1] < -0.0031)
            .collect();
        assert!(edge.iter().any(|v| v.position[0] > 0.0015));
        assert!(edge.iter().any(|v| v.position[0] < -0.0015));
        assert!(
            vertices
                .iter()
                .all(|v| v.uv[0] == -1. && (0.24..=0.320001).contains(&v.uv[1]))
        );
        let contact = CrownEnvelope {
            inverse: Mat4::IDENTITY,
            radii: Vec3::new(0.0023, 0.0034, 0.0016) + Vec3::splat(0.0002),
            tooth: 0,
            lower: false,
        };
        assert!(
            vertices
                .iter()
                .all(|v| contact.contains(Vec3::from_array(v.position))),
            "labial lobes must remain inside the tongue contact envelope"
        );
        SceneMesh::new(vertices, indices).unwrap();
    }
    #[test]
    fn posterior_crowns_have_inward_fissures_and_mirrored_cutting_faces() {
        let mut captured = Vec::new();
        for tooth in [3, 5] {
            let build = |lower| {
                let mut vertices = Vec::new();
                let mut indices = Vec::new();
                tooth_crown(
                    &mut vertices,
                    &mut indices,
                    Mat4::IDENTITY,
                    Vec3::new(0.0031, 0.0026, 0.0033),
                    tooth,
                    lower,
                );
                SceneMesh::new(vertices, indices).unwrap()
            };
            let upper = build(false);
            let lower = build(true);
            captured.push((
                tooth,
                lower
                    .vertices()
                    .iter()
                    .map(|v| v.position)
                    .collect::<Vec<_>>(),
                lower.indices().to_vec(),
            ));
            assert_eq!(upper.indices(), lower.indices());
            for mesh in [&upper, &lower] {
                let sign = if std::ptr::eq(mesh, &upper) { -1. } else { 1. };
                let center = mesh
                    .vertices()
                    .iter()
                    .find(|v| {
                        v.position[0].abs() < 1e-7
                            && v.position[2].abs() < 1e-7
                            && v.position[1] * sign > 0.
                    })
                    .unwrap();
                assert!((center.position[1].abs() - 0.0021).abs() < 1e-6);
                assert!(
                    mesh.vertices()
                        .iter()
                        .any(|v| v.position[1] * sign > 0.0023),
                    "cusps must stand above the central fissure"
                );
                assert!(
                    mesh.vertices()
                        .iter()
                        .all(|v| v.position[1].abs() <= 0.0026001)
                );
            }
        }
        if let Some(path) = std::env::var_os("VOXY_DENTAL_MESH_CAPTURE") {
            std::fs::write(path, serde_json::to_vec(&captured).unwrap()).unwrap();
        }
    }
    #[test]
    fn tongue_motion_preserves_topology_and_closed_mouth_rest() {
        let build = |jaw, lift, forward| {
            let mut vertices = Vec::new();
            let mut indices = Vec::new();
            append_oral_anatomy_pose(
                &mut vertices,
                &mut indices,
                Mat4::IDENTITY,
                crate::female_face::FacePose {
                    jaw,
                    tongue_lift: lift,
                    tongue_forward: forward,
                    ..Default::default()
                },
            );
            (vertices, indices)
        };
        assert_eq!(build(0., 0., 0.), build(0., 1., 1.));
        let rest = build(0.8, 0., 0.);
        let active = build(0.8, 0.7, 0.65);
        assert_eq!(rest.1, active.1);
        assert!(
            rest.0
                .iter()
                .zip(&active.0)
                .any(|(a, b)| b.position[2] - a.position[2] > 0.003)
        );
        assert!(
            active
                .0
                .iter()
                .all(|v| Vec3::from_array(v.position).is_finite())
        );
    }
    #[test]
    fn buccal_aperture_keeps_upper_edge_and_corners_fixed() {
        let front = |jaw| {
            let mut vertices = Vec::new();
            let mut indices = Vec::new();
            append_oral_anatomy(&mut vertices, &mut indices, Mat4::IDENTITY, jaw);
            vertices
                .iter()
                .filter(|v| v.uv == [-1., 0.42])
                .take(24)
                .map(|v| Vec3::from_array(v.position))
                .collect::<Vec<_>>()
        };
        let closed = front(0.);
        let open = front(1.);
        for segment in 0..=12 {
            assert!(
                closed[segment].distance(open[segment]) < 1e-7,
                "upper edge/corner moved at segment {segment}"
            );
        }
        assert!(closed[18].y - open[18].y > 0.009);
        assert!(closed[18].z - open[18].z > 0.0019);
    }
    #[test]
    fn buccal_lining_and_pharynx_share_the_same_boundary() {
        for jaw in [0., 0.45, 0.9, 1.] {
            let mut vertices = Vec::new();
            let mut indices = Vec::new();
            append_oral_anatomy(&mut vertices, &mut indices, Mat4::IDENTITY, jaw);
            let lining: Vec<_> = vertices.iter().filter(|v| v.uv == [-1., 0.42]).collect();
            let throat: Vec<_> = vertices.iter().filter(|v| v.uv == [-1., 0.55]).collect();
            assert_eq!(lining.len(), 9 * 24);
            assert_eq!(throat.len(), 9 * 24);
            for segment in 0..24 {
                let a = Vec3::from_array(lining[8 * 24 + segment].position);
                let b = Vec3::from_array(throat[segment].position);
                assert!(
                    a.distance(b) < 1e-7,
                    "jaw={jaw} segment={segment}: {a:?} vs {b:?}"
                );
            }
        }
    }
    #[test]
    fn oral_anatomy_keeps_topology_and_moves_lower_jaw() {
        let build = |jaw| {
            let mut vertices = Vec::new();
            let mut indices = Vec::new();
            append_oral_anatomy(&mut vertices, &mut indices, Mat4::IDENTITY, jaw);
            SceneMesh::new(vertices, indices).unwrap()
        };
        let closed = build(0.);
        let open = build(1.);
        assert_eq!(closed.indices(), open.indices());
        let fixed = closed
            .vertices()
            .iter()
            .zip(open.vertices())
            .filter(|(a, b)| a.position == b.position)
            .count();
        let lowered = closed
            .vertices()
            .iter()
            .zip(open.vertices())
            .filter(|(a, b)| a.position[1] - b.position[1] > 0.009)
            .count();
        assert!(
            fixed > 2500,
            "upper arch and palate should remain attached to skull"
        );
        assert!(lowered > 2500, "lower teeth and gingiva should follow jaw");
        assert!(
            open.vertices().iter().any(|v| v.position[2] < 0.085),
            "pharynx must have depth"
        );
    }
    #[test]
    fn split_upper_lip_does_not_inherit_posterior_lower_lip_roll() {
        let original = [
            Vec3::new(-0.001, 0.648, 0.150),
            Vec3::new(0.001, 0.648, 0.150),
            Vec3::new(0., 0.644, 0.150),
        ];
        let mut body: Vec<_> = original
            .iter()
            .map(|p| SceneVertex {
                position: p.to_array(),
                uv: [0.; 2],
                color: [1.; 4],
            })
            .collect();
        let pose = crate::female_face::FacePose {
            jaw: 1.,
            ..Default::default()
        };
        crate::female_face::deform(&mut body, 3, pose);
        let features = FaceFeatures {
            strands: Vec::new(),
            lash_motion: Vec::new(),
            seams: vec![([0, 1, 2], original)],
            rims: Vec::new(),
            lash_skin_vertices: Vec::new(),
            lash_skin_triangles: Vec::new(),
        };
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        features.append_lips(&body, &mut vertices, &mut indices, Mat4::IDENTITY, pose);
        let upper: Vec<_> = vertices.iter().filter(|v| v.position[1] > 0.645).collect();
        assert!(!upper.is_empty());
        assert!(
            upper.iter().all(|v| (v.position[2] - 0.150).abs() < 2e-7),
            "upper lip inherited lower-lip Z displacement"
        );
    }
    #[test]
    fn neutral_lip_split_preserves_original_surface_area() {
        let asset = ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            ObjLimits::default(),
        )
        .unwrap();
        let features = FaceFeatures::new(asset.mesh.vertices(), asset.mesh.indices());
        let area = |points: [Vec3; 3]| {
            (points[1] - points[0])
                .cross(points[2] - points[0])
                .length()
                * 0.5
        };
        let reference: f32 = features.seams.iter().map(|(_, p)| area(*p)).sum();
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        features.append_lips(
            asset.mesh.vertices(),
            &mut vertices,
            &mut indices,
            Mat4::IDENTITY,
            Default::default(),
        );
        let split: f32 = indices
            .chunks_exact(3)
            .map(|ids| {
                area(std::array::from_fn(|i| {
                    Vec3::from_array(vertices[ids[i] as usize].position)
                }))
            })
            .sum();
        assert!((reference - split).abs() / reference < 1e-5);
        SceneMesh::new(vertices, indices).unwrap();
    }
    #[test]
    fn opening_preserves_the_rest_of_the_source_mesh() {
        let asset = ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            ObjLimits::default(),
        )
        .unwrap();
        let original = asset.mesh.indices();
        let mut indices = original.to_vec();
        let count = open_mouth_surface(asset.mesh.vertices(), &mut indices);
        assert!(
            (50..400).contains(&count),
            "mouth aperture removed {count} triangles"
        );
        assert!(indices.len() > original.len() * 99 / 100);
        SceneMesh::new(asset.mesh.vertices().to_vec(), indices.clone()).unwrap();
        assert_eq!(open_mouth_surface(asset.mesh.vertices(), &mut indices), 0);
    }
    #[test]
    fn attachments_follow_surface_and_topology_is_stable() {
        let asset = ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            ObjLimits::default(),
        )
        .unwrap();
        let features = FaceFeatures::new(asset.mesh.vertices(), asset.mesh.indices());
        assert_eq!(features.strands.len(), 964);
        let lower: Vec<_> = features
            .strands
            .iter()
            .filter(|s| matches!(s.kind, Kind::LowerLash))
            .collect();
        assert_eq!(lower.len(), 48);
        for strand in lower {
            assert!(strand.direction.y < 0.);
            assert!(strand.side == -1. || strand.side == 1.);
            let root = strand
                .roots
                .iter()
                .enumerate()
                .map(|(k, &i)| {
                    Vec3::from_array(asset.mesh.vertices()[i].position) * strand.weights[k]
                })
                .sum::<Vec3>();
            assert_eq!(strand.side, root.x.signum());
            assert!(
                (0.018..=0.049).contains(&root.x.abs()),
                "lower lash escaped to nose or temple: {root:?}"
            );
            assert!(
                (0.704..=0.712).contains(&root.y),
                "lower lash escaped the lower rim: {root:?}"
            );
        }
        let normals = vec![Vec3::Z; asset.mesh.vertices().len()];
        let mut previous = None;
        for blink in [0., 0.5, 1.] {
            let mut vertices = Vec::new();
            let mut indices = Vec::new();
            features.append(
                asset.mesh.vertices(),
                &normals,
                &mut vertices,
                &mut indices,
                Mat4::IDENTITY,
                crate::female_face::FacePose {
                    blink,
                    jaw: 0.55,
                    ..Default::default()
                },
            );
            let mesh = SceneMesh::new(vertices, indices).unwrap();
            if let Some((v, i)) = previous {
                assert_eq!(mesh.vertices().len(), v);
                assert_eq!(mesh.indices().len(), i);
            }
            previous = Some((mesh.vertices().len(), mesh.indices().len()));
            assert_eq!(
                mesh.vertices()
                    .iter()
                    .filter(|v| v.uv == [-0.25, 0.45])
                    .count(),
                144 * 102
            );
            assert!(mesh.vertices().iter().all(|v| v.uv == [0.; 2]
                || v.uv == [-0.25, 0.45]
                || (v.uv[0] == -1. && (0.15..=0.9).contains(&v.uv[1]))
                || ((-1.4..=-1.2).contains(&v.uv[0]) && (0.0..=1.0).contains(&v.uv[1]))));
        }
    }
}

#[cfg(test)]
mod inertial_lash_regression {
    use super::*;
    #[test]
    fn lashes_respond_to_acceleration_and_settle_without_moving_the_attachment() {
        let strand = Strand {
            roots: [0, 1, 2],
            weights: Vec3::new(1., 0., 0.),
            direction: Vec3::Y * 0.005,
            radius: 0.00004,
            kind: Kind::Lash,
            side: 1.,
            bind_frame: Quat::IDENTITY,
        };
        let mut features = FaceFeatures {
            strands: vec![strand],
            lash_motion: vec![
                physics::secondary_motion::SecondaryMotion::new(
                    physics::secondary_motion::Config {
                        frequency: 24.,
                        damping_ratio: 0.3,
                    },
                )
                .unwrap(),
            ],
            seams: vec![],
            rims: vec![],
            lash_skin_vertices: vec![],
            lash_skin_triangles: vec![],
        };
        for _ in 0..12 {
            features.advance_lashes(1. / 240., [0., 20., 0.]).unwrap();
        }
        let peak = features.lash_motion[0].offset()[1].abs();
        assert!(peak > 1e-5 && peak < 0.001);
        for _ in 0..240 {
            features.advance_lashes(1. / 240., [0.; 3]).unwrap();
        }
        assert!(features.lash_motion[0].offset()[1].abs() < peak * 0.001);
        assert_eq!(features.strands[0].roots, strand.roots);
        assert_eq!(features.strands[0].weights, strand.weights);
    }
}
