//! Procedural expression offsets for the Blender female bind-pose mesh.
//! Model-specific masks are illustrative, rather than an anatomical muscle solver.
use glam::{Quat, Vec3};
use voxy_render::SceneVertex;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct FacePose {
    pub blink: f32,
    pub blink_offsets: [f32; 2],
    pub smile: f32,
    pub jaw: f32,
    pub tongue_lift: f32,
    pub tongue_forward: f32,
    pub brow: f32,
    pub squint: f32,
    pub gaze: [f32; 2],
    pub gaze_offsets: [[f32; 2]; 2],
}
impl FacePose {
    pub fn eye_rotation(self, x: f32) -> Quat {
        let offset = self.gaze_offsets[usize::from(x < 0.)];
        Quat::from_rotation_y((self.gaze[0] + offset[0]).clamp(-0.3, 0.3))
            * Quat::from_rotation_x((self.gaze[1] + offset[1]).clamp(-0.2, 0.2))
    }
    pub fn blink_for_side(self, x: f32) -> f32 {
        (self.blink + self.blink_offsets[usize::from(x < 0.)]).clamp(0., 1.)
    }
    pub fn sample(time: f32) -> Self {
        let t = time.rem_euclid(6.);
        // Shorter closure, slower reopening; parameters are preview design choices.
        let blink = |center: f32| {
            if t < center {
                smooth(center - 0.07, center, t)
            } else {
                1. - smooth(center, center + 0.16, t)
            }
        };
        let envelope = |start: f32, peak: f32, release: f32, end: f32| {
            smooth(start, peak, t) * (1. - smooth(release, end, t))
        };
        let smile = 0.62 * envelope(1.2, 2.8, 3.3, 5.3);
        let glance = |start: f32, end: f32| {
            smooth(start, start + 0.13, t) * (1. - smooth(end, end + 0.13, t))
        };
        let common = blink(0.9).max(blink(4.7));
        Self {
            blink: common,
            blink_offsets: [
                blink(0.894).max(blink(4.706)) - common,
                blink(0.906).max(blink(4.694)) - common,
            ],
            tongue_lift: 0.,
            tongue_forward: 0.,
            smile,
            jaw: 0.08 * envelope(3.2, 3.6, 3.9, 4.4),
            brow: 0.35 * envelope(0.35, 0.7, 1.1, 1.6),
            squint: 0.08 + 0.18 * smile,
            gaze_offsets: [[0.; 2]; 2],
            gaze: [
                0.065 * glance(0.68, 1.6) - 0.045 * glance(3.45, 4.35),
                -0.014 * glance(3.45, 4.35),
            ],
        }
    }
}
fn smooth(a: f32, b: f32, value: f32) -> f32 {
    let t = ((value - a) / (b - a)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}
fn region(p: Vec3, center: Vec3, radius: Vec3) -> f32 {
    let q = (p - center) / radius;
    (1. - q.length_squared()).max(0.).powi(2)
}
/// Bind-space line of lip contact, shared by clipping and jaw deformation.
pub(crate) fn mouth_seam(x: f32) -> f32 {
    let shoulder = (-((x.abs() - 0.007) / 0.005).powi(2)).exp();
    let notch = (-(x / 0.003).powi(2)).exp();
    0.6465 + 0.0007 * shoulder - 0.0002 * notch - 0.0015 * (x / 0.026).powi(2)
}
/// Lower-lip opening tapers to zero at the mouth corners. The surface is split
/// at this same seam by FaceFeatures, so upper and lower boundary weights differ.
pub(crate) fn jaw_weight(point: Vec3) -> f32 {
    if point.y >= mouth_seam(point.x) {
        return 0.;
    }
    let lip_width = 1. - smooth(0.019, 0.028, point.x.abs());
    let jaw_width = 1. - smooth(0.035, 0.070, point.x.abs());
    let lip_band = smooth(0.633, 0.642, point.y);
    smooth(0.592, 0.625, point.y) * (jaw_width * (1. - lip_band) + lip_width * lip_band)
}
/// Shared jaw and posterior-lip displacement before the front mask.
pub(crate) fn mouth_displacement(p: Vec3, jaw: f32) -> Vec3 {
    let jaw = jaw.clamp(0., 1.);
    let lower = jaw_weight(p);
    let inner_lip = smooth(0.632, 0.637, p.y)
        * (1. - smooth(0.645, 0.647, p.y))
        * smooth(0.130, 0.137, p.z)
        * (1. - smooth(0.147, 0.153, p.z))
        * (1. - smooth(0.019, 0.026, p.x.abs()));
    Vec3::new(
        0.,
        -0.010 * lower * jaw,
        -0.002 * lower * jaw - 0.004 * inner_lip * lower * jaw,
    )
}
/// Applied before skeletal skinning, so all offsets follow the animated head.
pub(crate) fn deform(vertices: &mut [SceneVertex], body_vertices: usize, pose: FacePose) {
    deform_with_contour(vertices, body_vertices, pose, None);
}

fn edge_compression(rest_length: f32, current_length: f32) -> f32 {
    if rest_length <= 1e-8 {
        return 0.;
    }
    ((rest_length - current_length) / rest_length).max(0.)
}

fn forehead_fold_depth(p: Vec3) -> f32 {
    let forehead = region(
        p,
        Vec3::new(0., 0.762, 0.132),
        Vec3::new(0.065, 0.043, 0.040),
    );
    if forehead <= 0. {
        return 0.;
    }
    let mut fold_depth = 0.;
    for (index, (level, width, length, depth)) in [
        (0.752, 0.0010, 0.052, 0.0008),
        (0.764, 0.0012, 0.047, 0.00065),
        (0.776, 0.0014, 0.041, 0.00045),
    ]
    .into_iter()
    .enumerate()
    {
        // Bind-space variation follows the skin; no frame-dependent noise.
        let phase = index as f32 * 1.7;
        let width = width * (0.9 + 0.1 * (p.x * 131. + phase).cos());
        let strength = 0.55 + 0.45 * (p.x * 97. + phase).sin().powi(2);
        let center = level
            + 0.002 * (p.x / length).powi(2)
            + 0.0004 * (p.x * 83. + phase).sin()
            + 0.00015 * (p.x * 211. - phase).sin();
        let distance = p.y - center;
        let groove = (-(distance / width).powi(2)).exp();
        let shoulder = (-((distance - 1.6 * width) / (1.3 * width)).powi(2)).exp();
        let lateral = 1. - smooth(length * 0.65, length, p.x.abs());
        fold_depth += depth * lateral * strength * (groove - 0.25 * shoulder);
    }
    forehead * fold_depth
}

const LID_SAMPLES: usize = 129;

/// Bind-space aperture samples from triangle/vertical-plane intersections.
#[derive(Debug)]
pub(crate) struct LidContour {
    edges: [[[f32; 2]; LID_SAMPLES]; 2],
    aperture: [[[f32; 2]; LID_SAMPLES]; 2],
    forehead_vertices: Vec<usize>,
    forehead_edges: Vec<[usize; 2]>,
}
impl LidContour {
    pub fn new(vertices: &[SceneVertex], indices: &[u32]) -> Self {
        let mut edges = [[[0.7104; 2]; LID_SAMPLES]; 2];
        for (side_index, side) in [1., -1.].into_iter().enumerate() {
            for (sample, pair) in edges[side_index].iter_mut().enumerate() {
                let x = side * (0.019 + 0.028 * sample as f32 / (LID_SAMPLES - 1) as f32);
                let mut lower = f32::NEG_INFINITY;
                let mut upper = f32::INFINITY;
                for ids in indices.chunks_exact(3) {
                    if ids.iter().any(|&i| i as usize >= vertices.len()) {
                        continue;
                    }
                    let triangle: [Vec3; 3] = std::array::from_fn(|i| {
                        Vec3::from_array(vertices[ids[i] as usize].position)
                    });
                    if triangle
                        .iter()
                        .any(|p| p.y < 0.700 || p.y > 0.725 || p.z < 0.120)
                    {
                        continue;
                    }
                    for edge in 0..3 {
                        let a = triangle[edge];
                        let b = triangle[(edge + 1) % 3];
                        if (a.x - b.x).abs() < 1e-9 || x < a.x.min(b.x) || x > a.x.max(b.x) {
                            continue;
                        }
                        let p = a.lerp(b, (x - a.x) / (b.x - a.x));
                        let front = 0.1215
                            + (0.0122_f32.powi(2)
                                - (p.x - side * 0.03287).powi(2)
                                - (p.y - 0.71242).powi(2))
                            .max(0.)
                            .sqrt();
                        if p.z < front - 0.0015 {
                            continue;
                        }
                        if p.y < 0.7104 {
                            lower = lower.max(p.y);
                        } else {
                            upper = upper.min(p.y);
                        }
                    }
                }
                if lower.is_finite() && upper.is_finite() {
                    *pair = [lower, upper];
                }
            }
        }
        let aperture = edges;
        // Regularize local edge samples before differentiating their displacement
        // through the surrounding skin; inner fold intersections can create spikes.
        for side in &mut edges {
            let source = *side;
            for (index, edge) in side.iter_mut().enumerate() {
                for channel in 0..2 {
                    let (mut total, mut weights) = (0., 0.);
                    // Keep approximately 0.875 mm smoothing sigma regardless
                    // of the finer aperture sampling interval (0.219 mm).
                    for offset in -8isize..=8 {
                        let weight = (-0.5 * (offset as f32 / 4.).powi(2)).exp();
                        let sample =
                            (index as isize + offset).clamp(0, (LID_SAMPLES - 1) as isize) as usize;
                        total += source[sample][channel] * weight;
                        weights += weight;
                    }
                    edge[channel] = total / weights;
                }
            }
        }
        let forehead_vertices: Vec<_> = vertices
            .iter()
            .enumerate()
            .filter_map(|(i, v)| {
                let p = v.position;
                (p[0].abs() < 0.07 && (0.735..0.795).contains(&p[1]) && p[2] > 0.11).then_some(i)
            })
            .collect();
        let local: std::collections::HashMap<_, _> = forehead_vertices
            .iter()
            .enumerate()
            .map(|(i, &id)| (id, i))
            .collect();
        let mut unique = std::collections::BTreeSet::new();
        for triangle in indices.chunks_exact(3) {
            for edge in [
                [triangle[0], triangle[1]],
                [triangle[1], triangle[2]],
                [triangle[2], triangle[0]],
            ] {
                if let (Some(&a), Some(&b)) = (
                    local.get(&(edge[0] as usize)),
                    local.get(&(edge[1] as usize)),
                ) {
                    if a != b {
                        unique.insert([a.min(b), a.max(b)]);
                    }
                }
            }
        }
        Self {
            edges,
            aperture,
            forehead_vertices,
            forehead_edges: unique.into_iter().collect(),
        }
    }
    pub(crate) fn at(&self, x: f32) -> [f32; 2] {
        Self::sample_edges(&self.edges, x)
    }
    pub(crate) fn aperture_at(&self, x: f32) -> [f32; 2] {
        Self::sample_edges(&self.aperture, x)
    }
    fn sample_edges(samples: &[[[f32; 2]; LID_SAMPLES]; 2], x: f32) -> [f32; 2] {
        let coordinate = (((x.abs() - 0.019) / 0.028).clamp(0., 1.)) * (LID_SAMPLES - 1) as f32;
        let index = (coordinate as usize).min(LID_SAMPLES - 2);
        let t = coordinate - index as f32;
        let edges = &samples[usize::from(x < 0.)];
        std::array::from_fn(|axis| edges[index][axis] * (1. - t) + edges[index + 1][axis] * t)
    }
}
pub(crate) fn deform_with_contour(
    vertices: &mut [SceneVertex],
    body_vertices: usize,
    pose: FacePose,
    contour: Option<&LidContour>,
) {
    let smile = pose.smile.clamp(0., 1.);
    let jaw = pose.jaw.clamp(0., 1.);
    let brow = pose.brow.clamp(-1., 1.);
    let squint = pose.squint.clamp(0., 0.5);
    let use_tension = contour.is_some()
        && std::env::var("VOXY_FACE_DIAGNOSTIC_NO_FOREHEAD_TENSION").as_deref() != Ok("1");
    let tension_rest: Option<Vec<Vec3>> = contour.filter(|_| brow > 0. && use_tension).map(|c| {
        c.forehead_vertices
            .iter()
            .map(|&i| Vec3::from_array(vertices[i].position))
            .collect()
    });
    for (index, vertex) in vertices.iter_mut().enumerate() {
        let p = Vec3::from_array(vertex.position);
        let blink = pose.blink_for_side(p.x);
        if index >= body_vertices {
            let center = crate::female_eyes::center(p.x);
            let rotation = pose.eye_rotation(p.x);
            vertex.position = (center + rotation * (p - center)).to_array();
            continue;
        }
        let front = smooth(0.08, 0.12, p.z);
        let mut delta = Vec3::ZERO;
        // Raise mouth corners and cheek pads without affecting nose or rear head.
        for side in [-1., 1.] {
            let corner = region(
                p,
                Vec3::new(side * 0.023, 0.647, 0.145),
                Vec3::new(0.024, 0.022, 0.04),
            );
            delta += Vec3::new(side * 0.003, 0.004, 0.001) * corner * smile;
            let brow_mask = region(
                p,
                Vec3::new(side * 0.033, 0.738, 0.13),
                Vec3::new(0.031, 0.018, 0.03),
            );
            delta.y += 0.005 * brow_mask * brow;
            let frown = (-brow).max(0.);
            let inner_brow = 1. - smooth(0.020, 0.045, p.x.abs());
            delta.x -= side * 0.0015 * brow_mask * inner_brow * frown;
            let cheek = region(
                p,
                Vec3::new(side * 0.042, 0.692, 0.134),
                Vec3::new(0.032, 0.025, 0.035),
            );
            delta += Vec3::new(side * 0.0008, 0.0025, 0.001) * cheek * smile;
            // The upper lid travels farther; both rims glide to a shared lower seam.
            let dx = (p.x - side * 0.03287).abs();
            let dy = p.y - 0.71242;
            let base_depth = smooth(0.118, 0.129, p.z);
            let depth = contour.map_or(base_depth, |contour| {
                let aperture = contour.aperture_at(p.x);
                let distance = (p.y - aperture[0]).abs().min((p.y - aperture[1]).abs());
                let rim = (1. - smooth(0.0003, 0.0015, distance)) * smooth(0.120, 0.126, p.z);
                base_depth + (1. - base_depth) * rim
            });
            let lid =
                (1. - smooth(0.014, 0.025, dx)) * (1. - smooth(0.007, 0.022, dy.abs())) * depth;
            let narrowed = blink + (1. - blink) * squint;
            let seam = 0.7104;
            let lid_shift = if let Some(contour) = contour {
                let edge = contour.at(p.x);
                let upper = p.y >= seam;
                let aperture = contour.aperture_at(p.x);
                let raw_boundary = if upper { aperture[1] } else { aperture[0] };
                let raw_outside = if upper {
                    (p.y - raw_boundary).max(0.)
                } else {
                    (raw_boundary - p.y).max(0.)
                };
                let filtered_boundary = if upper { edge[1] } else { edge[0] };
                // Preserve the actual contact boundary; blend toward the smooth
                // profile only in the surrounding skin, away from the rim.
                let blend = smooth(0.0003, 0.002, raw_outside);
                let boundary = raw_boundary + (filtered_boundary - raw_boundary) * blend;
                let outside = if upper {
                    (p.y - boundary).max(0.)
                } else {
                    (boundary - p.y).max(0.)
                };
                let width = if upper { 0.006 } else { 0.003 };
                let falloff = (-(outside / width).powi(2)).exp();
                let horizontal =
                    smooth(0.018, 0.021, p.x.abs()) * (1. - smooth(0.045, 0.048, p.x.abs()));
                let target = if outside > 0. {
                    p.y + (seam - boundary) * falloff
                } else {
                    seam
                };
                (target - p.y) * lid * narrowed * horizontal
            } else {
                (seam - p.y) * lid * narrowed
            };
            delta.y += lid_shift;
            let dx_signed = p.x - side * 0.03287;
            let moved_y = p.y + lid_shift;
            let globe_radius = 0.0122;
            let sphere_z = 0.1215
                + (globe_radius * globe_radius
                    - dx_signed * dx_signed
                    - (moved_y - 0.71242).powi(2))
                .max(0.)
                .sqrt()
                + 0.0004;
            if let Some(contour) = contour {
                // Preserve bind-space separation between shell layers while
                // carrying them along the globe's changed vertical curvature.
                let bind_shell = 0.1215
                    + (globe_radius * globe_radius - dx_signed * dx_signed - dy * dy)
                        .max(0.)
                        .sqrt()
                    + 0.0004;
                let aperture = contour.aperture_at(p.x);
                let distance = (p.y - aperture[0]).abs().min((p.y - aperture[1]).abs());
                let contact = (1. - smooth(0.0003, 0.0015, distance)) * smooth(0.120, 0.126, p.z);
                let separation = p.z - bind_shell;
                // Limit anterior correction instead of flattening posterior layers
                // against one depth, which can compress and invert the lid folds.
                let separation = separation + (-separation).clamp(0., 0.00015) * contact;
                let target_z = sphere_z + separation;
                // Glide the actual rim over the globe, then release surrounding
                // skin from the spherical constraint instead of bulging its fold.
                let glide = 1. - smooth(0.0015, 0.004, distance);
                delta.z += (target_z - p.z) * lid * narrowed * glide;
            } else {
                delta.z += (sphere_z - p.z).max(0.) * lid * narrowed;
            }
        }
        let forehead = region(
            p,
            Vec3::new(0., 0.762, 0.132),
            Vec3::new(0.065, 0.043, 0.040),
        );
        delta.y += 0.002 * forehead * brow;
        if !use_tension && brow > 0. {
            delta.z -= brow * forehead_fold_depth(p);
        }
        let glabella = region(
            p,
            Vec3::new(0., 0.741, 0.132),
            Vec3::new(0.018, 0.025, 0.040),
        );
        let frown_lines = (-((p.x.abs() - 0.004) / 0.0013).powi(2)).exp();
        delta.z -= 0.00035 * glabella * (-brow).max(0.) * frown_lines;
        delta += mouth_displacement(p, jaw);
        vertex.position = (p + delta * front).to_array();
    }
    if let (Some(c), Some(rest)) = (contour, tension_rest) {
        let mut compression = vec![0.; rest.len()];
        let mut counts = vec![0_u32; rest.len()];
        for &[a, b] in &c.forehead_edges {
            let d0 = rest[a].distance(rest[b]);
            if d0 <= 1e-8 {
                continue;
            }
            let d1 = Vec3::from_array(vertices[c.forehead_vertices[a]].position)
                .distance(Vec3::from_array(vertices[c.forehead_vertices[b]].position));
            let strain = edge_compression(d0, d1);
            for i in [a, b] {
                compression[i] += strain;
                counts[i] += 1;
            }
        }
        if std::env::var("VOXY_FACE_DIAGNOSTIC_FOREHEAD_STRAIN").as_deref() == Ok("1") {
            let values: Vec<_> = compression
                .iter()
                .enumerate()
                .map(|(i, &v)| v / counts[i].max(1) as f32)
                .collect();
            eprintln!(
                "FOREHEAD STRAIN brow={brow} max={} mean={}",
                values.iter().copied().fold(0_f32, f32::max),
                values.iter().sum::<f32>() / values.len().max(1) as f32
            );
        }
        for (i, &id) in c.forehead_vertices.iter().enumerate() {
            // Average avoids valence-dependent wrinkle strength. 1.5% is an
            // artistic activation scale, not a biomechanical tissue constant.
            let activation = (compression[i] / counts[i].max(1) as f32 / 0.015).clamp(0., 1.);
            vertices[id].position[2] -=
                activation * forehead_fold_depth(rest[i]) * smooth(0.08, 0.12, rest[i].z);
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    #[test]
    fn tension_ignores_rigid_motion_and_stretch_and_measures_compression() {
        let edge = Vec3::new(0.001, 0.002, 0.0003);
        let rotated = glam::Quat::from_rotation_z(0.7) * edge;
        assert!(edge_compression(edge.length(), rotated.length()) < 1e-6);
        assert_eq!(edge_compression(edge.length(), edge.length() * 1.1), 0.);
        assert!((edge_compression(edge.length(), edge.length() * 0.9) - 0.1).abs() < 1e-6);
        assert_eq!(edge_compression(0., 0.), 0.);
    }
    #[test]
    fn frown_draws_inner_brows_inward_without_changing_neutral_or_outer_points() {
        let source = [-0.023, 0.023, 0.08].map(|x| SceneVertex {
            position: [x, 0.741, 0.134],
            uv: [0.; 2],
            color: [1.; 4],
        });
        let mut neutral = source;
        deform(&mut neutral, 3, FacePose::default());
        assert_eq!(neutral.map(|v| v.position), source.map(|v| v.position));
        let mut frown = source;
        deform(
            &mut frown,
            3,
            FacePose {
                brow: -1.,
                ..Default::default()
            },
        );
        for i in 0..2 {
            let shift = source[i].position[0].abs() - frown[i].position[0].abs();
            assert!((0.0002..0.0015).contains(&shift));
            assert_eq!(
                source[i].position[0].signum(),
                frown[i].position[0].signum()
            );
        }
        assert_eq!(frown[2].position[0], source[2].position[0]);
    }
    /// Orthographic bind-space visibility diagnostic, not a closure gate.
    #[test]
    #[ignore = "writes sampled actual-globe/skin depth visibility"]
    fn audit_lid_globe_coverage() {
        fn front_depth(point: glam::Vec2, triangles: &[[Vec3; 3]]) -> Option<f32> {
            let mut depth: Option<f32> = None;
            for &[a, b, c] in triangles {
                if point.x < a.x.min(b.x).min(c.x)
                    || point.x > a.x.max(b.x).max(c.x)
                    || point.y < a.y.min(b.y).min(c.y)
                    || point.y > a.y.max(b.y).max(c.y)
                {
                    continue;
                }
                let ab = (b - a).truncate();
                let ac = (c - a).truncate();
                let determinant = ab.perp_dot(ac);
                if determinant.abs() < 1e-14 {
                    continue;
                }
                let ap = point - a.truncate();
                let u = ap.perp_dot(ac) / determinant;
                let v = ab.perp_dot(ap) / determinant;
                if u < -1e-5 || v < -1e-5 || u + v > 1.00001 {
                    continue;
                }
                let z = a.z + u * (b.z - a.z) + v * (c.z - a.z);
                depth = Some(depth.map_or(z, |old| old.max(z)));
            }
            depth
        }
        let body = ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            ObjLimits::default(),
        )
        .unwrap();
        let contour = super::LidContour::new(body.mesh.vertices(), body.mesh.indices());
        let mut csv = String::from("closure,side,x,y,globe_z,skin_z,rim_z,globe_visible\n");
        let include_rims = std::env::var("VOXY_LID_AUDIT_RIMS").as_deref() == Ok("1");
        let features = include_rims.then(|| {
            crate::female_features::FaceFeatures::new(body.mesh.vertices(), body.mesh.indices())
        });
        let fine = std::env::var("VOXY_LID_AUDIT_FINE").as_deref() == Ok("1");
        let (columns, rows, step) = if fine {
            (224, 160, 0.000125)
        } else {
            (56, 40, 0.0005)
        };
        let closures: &[f32] = if fine { &[1.] } else { &[0., 0.5, 1.] };
        for &closure in closures {
            let mut skin = body.mesh.vertices().to_vec();
            super::deform_with_contour(
                &mut skin,
                body.mesh.vertices().len(),
                super::FacePose {
                    blink: closure,
                    ..Default::default()
                },
                Some(&contour),
            );
            let skin_triangles: Vec<_> = body
                .mesh
                .indices()
                .chunks_exact(3)
                .filter_map(|ids| {
                    let tri: [Vec3; 3] =
                        std::array::from_fn(|k| Vec3::from_array(skin[ids[k] as usize].position));
                    tri.iter()
                        .all(|p| p.y > 0.69 && p.y < 0.74 && p.z > 0.10)
                        .then_some(tri)
                })
                .collect();
            let mut rim_triangles = Vec::new();
            if let Some(features) = &features {
                let mut normals = vec![Vec3::ZERO; skin.len()];
                for ids in body.mesh.indices().chunks_exact(3) {
                    let [a, b, c] =
                        std::array::from_fn(|k| Vec3::from_array(skin[ids[k] as usize].position));
                    let normal = (b - a).cross(c - a);
                    for &id in ids {
                        normals[id as usize] += normal;
                    }
                }
                for normal in &mut normals {
                    *normal = normal.try_normalize().unwrap_or(Vec3::Z);
                }
                let (mut vertices, mut indices) = (Vec::new(), Vec::new());
                features.append(
                    &skin,
                    &normals,
                    &mut vertices,
                    &mut indices,
                    glam::Mat4::IDENTITY,
                    super::FacePose {
                        blink: closure,
                        ..Default::default()
                    },
                );
                for ids in indices.chunks_exact(3) {
                    let triangle: [voxy_render::SceneVertex; 3] =
                        std::array::from_fn(|k| vertices[ids[k] as usize]);
                    if triangle.iter().all(|v| {
                        v.uv == [-1., 0.24] && v.position[1] > 0.69 && v.position[1] < 0.74
                    }) {
                        rim_triangles.push(triangle.map(|v| Vec3::from_array(v.position)));
                    }
                }
            }
            for (side, source) in [
                (
                    1.,
                    include_str!("../../../assets/characters/blender-female/eye-l.obj"),
                ),
                (
                    -1.,
                    include_str!("../../../assets/characters/blender-female/eye-r.obj"),
                ),
            ] {
                let eye = ObjAsset::parse(source, ObjLimits::default()).unwrap();
                let (vertices, indices) = crate::female_eyes::refined_globe(&eye.mesh);
                let globe: Vec<[Vec3; 3]> = indices
                    .chunks_exact(3)
                    .map(|ids| {
                        std::array::from_fn(|k| {
                            Vec3::from_array(vertices[ids[k] as usize].position)
                        })
                    })
                    .collect();
                let (mut samples, mut visible) = (0, 0);
                for column in 0..=columns {
                    for row in 0..=rows {
                        let x = side * (0.019 + column as f32 * step);
                        let y = 0.702 + row as f32 * step;
                        let point = glam::Vec2::new(x, y);
                        let Some(globe_z) = front_depth(point, &globe) else {
                            continue;
                        };
                        let skin_z = front_depth(point, &skin_triangles);
                        let rim_z = front_depth(point, &rim_triangles);
                        let exposed = skin_z.is_none_or(|z| z < globe_z - 0.00001)
                            && rim_z.is_none_or(|z| z < globe_z - 0.00001);
                        samples += 1;
                        visible += usize::from(exposed);
                        csv.push_str(&format!(
                            "{closure},{side},{x},{y},{globe_z},{},{},{}\n",
                            skin_z.map_or(String::new(), |z| z.to_string()),
                            rim_z.map_or(String::new(), |z| z.to_string()),
                            u8::from(exposed)
                        ));
                    }
                }
                println!(
                    "ACTUAL GLOBE COVERAGE closure={closure} side={side}: visible={visible}/{samples}"
                );
            }
        }
        std::fs::write(
            if fine && include_rims {
                "/tmp/voxy-lid-globe-coverage-rims-fine.csv"
            } else if fine {
                "/tmp/voxy-lid-globe-coverage-fine.csv"
            } else if include_rims {
                "/tmp/voxy-lid-globe-coverage-rims.csv"
            } else {
                "/tmp/voxy-lid-globe-coverage.csv"
            },
            csv,
        )
        .unwrap();
    }

    // Strict transverse intersection only: excludes shared edges, endpoint
    // touches and coplanar overlap. f64 keeps metre-scale predicates stable.
    pub(crate) fn segment_crosses_triangle(
        a: glam::DVec3,
        b: glam::DVec3,
        tri: [glam::DVec3; 3],
    ) -> bool {
        let direction = b - a;
        let e1 = tri[1] - tri[0];
        let e2 = tri[2] - tri[0];
        let h = direction.cross(e2);
        let determinant = e1.dot(h);
        if determinant.abs() < 1e-12 * direction.length() * e1.length() * e2.length() {
            return false;
        }
        if determinant == 0. {
            return false;
        }
        let offset = a - tri[0];
        let u = offset.dot(h) / determinant;
        let q = offset.cross(e1);
        let v = direction.dot(q) / determinant;
        let t = e2.dot(q) / determinant;
        let epsilon = 1e-7;
        u > epsilon && v > epsilon && u + v < 1. - epsilon && t > epsilon && t < 1. - epsilon
    }
    pub(crate) fn triangles_cross(a: [Vec3; 3], b: [Vec3; 3]) -> bool {
        for axis in 0..3 {
            let bounds = |t: [Vec3; 3]| {
                (
                    t.iter().map(|p| p[axis]).fold(f32::INFINITY, f32::min),
                    t.iter().map(|p| p[axis]).fold(f32::NEG_INFINITY, f32::max),
                )
            };
            let (amin, amax) = bounds(a);
            let (bmin, bmax) = bounds(b);
            if amax < bmin || bmax < amin {
                return false;
            }
        }
        for k in 0..3 {
            if segment_crosses_triangle(
                a[k].as_dvec3(),
                a[(k + 1) % 3].as_dvec3(),
                b.map(Vec3::as_dvec3),
            ) || segment_crosses_triangle(
                b[k].as_dvec3(),
                b[(k + 1) % 3].as_dvec3(),
                a.map(Vec3::as_dvec3),
            ) {
                return true;
            }
        }
        false
    }
    #[test]
    fn transverse_intersection_diagnostic_rejects_touches_and_separation() {
        let a = [Vec3::ZERO, Vec3::X, Vec3::Y];
        let b = [
            Vec3::new(0.2, 0.2, -1.),
            Vec3::new(0.2, 0.2, 1.),
            Vec3::new(0.8, 0.2, 1.),
        ];
        assert!(triangles_cross(a, b));
        assert!(triangles_cross(b, a));
        assert!(!triangles_cross(a, b.map(|p| p + Vec3::X * 2.)));
        assert!(!triangles_cross(a, a));
        assert!(!triangles_cross(a, [Vec3::ZERO, Vec3::X, Vec3::Z]));
    }
    #[test]
    #[ignore = "writes strict transverse skin intersection measurements"]
    fn audit_lid_self_intersections() {
        use std::collections::BTreeSet;
        let asset = ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            ObjLimits::default(),
        )
        .unwrap();
        let rest = asset.mesh.vertices();
        let contour = LidContour::new(rest, asset.mesh.indices());
        let selected: Vec<_> = asset
            .mesh
            .indices()
            .chunks_exact(3)
            .enumerate()
            .filter_map(|(index, ids)| {
                let ids: [usize; 3] = std::array::from_fn(|k| ids[k] as usize);
                let tri = ids.map(|i| Vec3::from_array(rest[i].position));
                tri.iter()
                    .all(|p| {
                        (0.014..0.052).contains(&p.x.abs())
                            && (0.694..0.735).contains(&p.y)
                            && p.z >= 0.118
                    })
                    .then_some((index, ids, tri))
            })
            .collect();
        let intersections = |vertices: &[voxy_render::SceneVertex]| {
            let triangles: Vec<_> = selected
                .iter()
                .map(|(_, ids, _)| ids.map(|i| Vec3::from_array(vertices[i].position)))
                .collect();
            let mut hits = BTreeSet::new();
            for i in 0..selected.len() {
                for j in i + 1..selected.len() {
                    // OBJ UV seams may duplicate indices: compare bind positions.
                    if selected[i].2.iter().any(|p| selected[j].2.contains(p)) {
                        continue;
                    }
                    if triangles_cross(triangles[i], triangles[j]) {
                        hits.insert((selected[i].0, selected[j].0));
                    }
                }
            }
            hits
        };
        let original = intersections(rest);
        assert!(
            original.is_empty(),
            "reference skin contains transverse overlaps"
        );
        let mut csv = String::from("mode,closure,transverse_pairs,new_pairs_relative_to_rest\n");
        let mut details = String::from("mode,triangle_a,triangle_b\n");
        for (mode, profile) in [("baseline", None), ("contour", Some(&contour))] {
            for closure in [0., 0.5, 0.8, 1.] {
                let mut posed = rest.to_vec();
                deform_with_contour(
                    &mut posed,
                    rest.len(),
                    FacePose {
                        blink: closure,
                        ..Default::default()
                    },
                    profile,
                );
                let hits = intersections(&posed);
                csv.push_str(&format!(
                    "{mode},{closure},{},{}\n",
                    hits.len(),
                    hits.difference(&original).count()
                ));
                if closure == 1. {
                    for &(a, b) in hits.difference(&original) {
                        details.push_str(&format!("{mode},{a},{b}\n"));
                    }
                }
            }
        }
        std::fs::write("/tmp/voxy-lid-intersections.csv", &csv).unwrap();
        std::fs::write("/tmp/voxy-lid-intersection-pairs.csv", details).unwrap();
        println!("{csv}");
    }
    /// Explicit diagnostic, not a gate claiming anatomically valid closure.
    #[test]
    #[ignore = "writes the bind/posed triangle audit for manual inspection"]
    fn audit_lid_triangle_orientation() {
        let asset = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let rest = asset.mesh.vertices();
        let contour = super::LidContour::new(rest, asset.mesh.indices());
        let mut csv = String::from(
            "mode,closure,triangles,min_area_ratio,area_below_1_percent,normal_reversals,projected_reversals\n",
        );
        let mut details = String::from("mode,triangle,x,y,z,area_ratio,normal_dot\n");
        for (mode, profile) in [("baseline", None), ("contour", Some(&contour))] {
            for step in 0..=10 {
                let closure = step as f32 / 10.;
                let mut posed = rest.to_vec();
                super::deform_with_contour(
                    &mut posed,
                    rest.len(),
                    super::FacePose {
                        blink: closure,
                        ..Default::default()
                    },
                    profile,
                );
                let (mut count, mut tiny, mut reversed, mut projected) = (0, 0, 0, 0);
                let mut minimum = f32::INFINITY;
                for (index, ids) in asset.mesh.indices().chunks_exact(3).enumerate() {
                    let a: [glam::Vec3; 3] = std::array::from_fn(|k| {
                        glam::Vec3::from_array(rest[ids[k] as usize].position)
                    });
                    if a.iter().any(|p| {
                        !(0.014..0.052).contains(&p.x.abs())
                            || !(0.694..0.735).contains(&p.y)
                            || p.z < 0.118
                    }) {
                        continue;
                    }
                    let b: [glam::Vec3; 3] = std::array::from_fn(|k| {
                        glam::Vec3::from_array(posed[ids[k] as usize].position)
                    });
                    let original = (a[1] - a[0]).cross(a[2] - a[0]);
                    let deformed = (b[1] - b[0]).cross(b[2] - b[0]);
                    if original.length() < 1e-12 {
                        continue;
                    }
                    let ratio = deformed.length() / original.length();
                    let dot = original
                        .normalize()
                        .dot(deformed.try_normalize().unwrap_or(glam::Vec3::ZERO));
                    assert!(ratio.is_finite() && dot.is_finite());
                    count += 1;
                    minimum = minimum.min(ratio);
                    tiny += usize::from(ratio < 0.01);
                    reversed += usize::from(dot < 0.);
                    projected += usize::from(original.z * deformed.z < 0.);
                    if step == 10 && (ratio < 0.01 || dot < 0.) {
                        let center = (a[0] + a[1] + a[2]) / 3.;
                        details.push_str(&format!(
                            "{mode},{index},{},{},{},{ratio},{dot}\n",
                            center.x, center.y, center.z
                        ));
                    }
                }
                csv.push_str(&format!(
                    "{mode},{closure},{count},{minimum},{tiny},{reversed},{projected}\n"
                ));
            }
        }
        std::fs::write("/tmp/voxy-lid-topology.csv", &csv).unwrap();
        std::fs::write("/tmp/voxy-lid-topology-triangles.csv", details).unwrap();
        println!("{csv}");
    }
    #[test]
    fn source_aperture_contour_tracks_both_rims_and_preserves_lid_band() {
        let asset = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let contour = super::LidContour::new(asset.mesh.vertices(), asset.mesh.indices());
        for x in [-0.033, 0.033] {
            let [lower, upper] = contour.at(x);
            assert!((0.706..0.709).contains(&lower), "lower rim {lower}");
            assert!((0.716..0.719).contains(&upper), "upper rim {upper}");
            let mut band = [upper, upper + 0.002].map(|y| voxy_render::SceneVertex {
                position: [x, y, 0.134],
                uv: [0.; 2],
                color: [1.; 4],
            });
            super::deform_with_contour(
                &mut band,
                2,
                super::FacePose {
                    blink: 1.,
                    ..Default::default()
                },
                Some(&contour),
            );
            assert!(band[1].position[1] - band[0].position[1] > 0.0018);
            let mut layers = [0.133, 0.134].map(|z| voxy_render::SceneVertex {
                position: [x, upper, z],
                uv: [0.; 2],
                color: [1.; 4],
            });
            super::deform_with_contour(
                &mut layers,
                2,
                super::FacePose {
                    blink: 1.,
                    ..Default::default()
                },
                Some(&contour),
            );
            assert!(
                (layers[1].position[2] - layers[0].position[2] - 0.001).abs() < 1e-6,
                "closure collapsed separated shell layers"
            );
        }
        for x in [-0.042, -0.034, -0.026, 0.026, 0.034, 0.042] {
            let aperture = contour.aperture_at(x);
            for depth in [0.127, 0.129, 0.134] {
                let mut rim = aperture.map(|y| voxy_render::SceneVertex {
                    position: [x, y, depth],
                    uv: [0.; 2],
                    color: [1.; 4],
                });
                super::deform_with_contour(
                    &mut rim,
                    2,
                    super::FacePose {
                        blink: 1.,
                        ..Default::default()
                    },
                    Some(&contour),
                );
                assert!(
                    (rim[1].position[1] - rim[0].position[1]).abs() < 0.00005,
                    "sampled contact rim remained open at x={x}, depth={depth}: {rim:?}"
                );
            }
        }
        for side in contour.edges {
            assert!(
                side.iter()
                    .all(|e| e[0].is_finite() && e[1].is_finite() && e[0] <= e[1])
            );
        }
    }
    #[test]
    fn blink_sides_are_bounded_offset_and_manually_close_together() {
        let manual = super::FacePose {
            blink: 1.,
            ..Default::default()
        };
        assert_eq!(manual.blink_for_side(1.), 1.);
        assert_eq!(manual.blink_for_side(-1.), 1.);
        let closing = super::FacePose::sample(0.86);
        assert!(closing.blink_for_side(1.) > closing.blink_for_side(-1.));
        let second = super::FacePose::sample(4.66);
        assert!(second.blink_for_side(1.) < second.blink_for_side(-1.));
        for step in 0..600 {
            let p = super::FacePose::sample(step as f32 * 0.01);
            for side in [-1., 1.] {
                assert!((0. ..=1.).contains(&p.blink_for_side(side)));
            }
        }
    }
    use super::*;
    use voxy_render::{ObjAsset, ObjLimits};
    #[test]
    fn jaw_opens_lower_lip_without_folding_it_or_dragging_upper_lip() {
        assert_eq!(jaw_weight(Vec3::new(0., 0.6466, 0.145)), 0.);
        assert_eq!(jaw_weight(Vec3::new(0.028, 0.6464, 0.145)), 0.);
        let top = jaw_weight(Vec3::new(0., mouth_seam(0.) - 0.0001, 0.145));
        let bottom = jaw_weight(Vec3::new(0., 0.638, 0.145));
        assert!((top - 1.).abs() < 1e-6 && (bottom - 1.).abs() < 1e-6);
        let left = jaw_weight(Vec3::new(-0.023, mouth_seam(-0.023) - 0.0001, 0.145));
        let right = jaw_weight(Vec3::new(0.023, mouth_seam(0.023) - 0.0001, 0.145));
        assert_eq!(left, right);
        assert!(left > 0. && left < 1.);
    }
    #[test]
    fn expressions_are_local_and_neutral_is_identity() {
        let asset = ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            ObjLimits::default(),
        )
        .unwrap();
        let original = asset.mesh.vertices();
        let mut vertices = original.to_vec();
        let count = vertices.len();
        deform(&mut vertices, count, FacePose::default());
        assert!(
            vertices
                .iter()
                .zip(original)
                .all(|(a, b)| a.position == b.position)
        );
        deform(
            &mut vertices,
            count,
            FacePose {
                blink: 1.,
                smile: 1.,
                jaw: 1.,
                brow: 1.,
                ..FacePose::default()
            },
        );
        let mut changed = 0;
        for (a, b) in vertices.iter().zip(original) {
            let p = Vec3::from_array(b.position);
            let displacement = Vec3::from_array(a.position).distance(p);
            assert!(displacement.is_finite() && displacement < 0.022);
            if p.y < 0.59 || p.z < 0.08 {
                assert_eq!(a.position, b.position);
            }
            if displacement > 1e-6 {
                changed += 1;
            }
        }
        assert!(changed > 300, "expression masks missed face: {changed}");
    }
    #[test]
    fn forehead_moves_with_brows_and_restores_without_accumulation() {
        let asset = ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            ObjLimits::default(),
        )
        .unwrap();
        let original = asset.mesh.vertices();
        let mut raised = original.to_vec();
        deform(
            &mut raised,
            original.len(),
            FacePose {
                brow: 0.35,
                ..Default::default()
            },
        );
        let changed = original
            .iter()
            .zip(&raised)
            .filter(|(a, b)| {
                a.position[1] > 0.75
                    && Vec3::from_array(a.position).distance(Vec3::from_array(b.position)) > 0.0001
            })
            .count();
        assert!(changed > 100, "forehead did not follow brows: {changed}");
        let mut neutral = original.to_vec();
        deform(&mut neutral, original.len(), Default::default());
        assert!(
            neutral
                .iter()
                .zip(original)
                .all(|(a, b)| a.position == b.position)
        );
    }
    #[test]
    fn gaze_preserves_eyeball_shape() {
        let asset = ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/eye-l.obj"),
            ObjLimits::default(),
        )
        .unwrap();
        let original = asset.mesh.vertices();
        let mut vertices = original.to_vec();
        deform(
            &mut vertices,
            0,
            FacePose {
                gaze: [0.2, -0.1],
                ..FacePose::default()
            },
        );
        let center = crate::female_eyes::center(1.);
        let mut changed = 0;
        for (a, b) in vertices.iter().zip(original) {
            let before = Vec3::from_array(b.position);
            let after = Vec3::from_array(a.position);
            assert!((after.distance(center) - before.distance(center)).abs() < 1e-6);
            if before.distance(after) > 1e-6 {
                changed += 1;
            }
        }
        assert!(changed > 500);
    }
    #[test]
    fn expression_channels_have_distinct_timing_and_gaze_holds() {
        let attentive = FacePose::sample(0.9);
        let smiling = FacePose::sample(3.);
        assert!(attentive.brow > 0. && attentive.smile == 0.);
        assert!(smiling.smile > 0.5 && smiling.brow == 0. && smiling.jaw == 0.);
        assert!(smiling.squint > 0.);
        assert_eq!(FacePose::sample(1.).gaze, FacePose::sample(1.4).gaze);
        assert!(FacePose::sample(0.85).blink < FacePose::sample(0.95).blink);
    }
    #[test]
    fn loop_is_continuous_and_blinks_close_then_reopen() {
        assert_eq!(FacePose::sample(0.9).blink, 1.);
        assert_eq!(FacePose::sample(1.1).blink, 0.);
        let a = FacePose::sample(0.);
        let b = FacePose::sample(6.);
        assert_eq!(a.smile, b.smile);
        assert_eq!(a.gaze, b.gaze);
        assert!((FacePose::sample(5.9999).gaze[0] - a.gaze[0]).abs() < 0.0001);
    }
}
