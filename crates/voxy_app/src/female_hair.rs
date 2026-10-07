//! Cosserat guide rods with contacts against the animated anatomical surface.
use glam::{Mat4, Quat, Vec3};
use physics::hair::{HairMaterial, HairRod, HairSystem, RootPose, TriangleMesh};
use std::collections::BTreeMap;
use voxy_render::SceneVertex;
const FOLLOWERS: usize = 60;

#[derive(Debug)]
pub(crate) struct FemaleHair {
    system: HairSystem,
    #[cfg(test)]
    roots: Vec<Vec3>,
    root_bindings: Vec<(usize, Vec3)>,
    targets: Vec<Vec3>,
    collider: TriangleMesh,
    render_offsets: Vec<Vec<Vec3>>,
}
impl FemaleHair {
    pub fn new(body: &[SceneVertex], indices: &[u32]) -> Result<Self, &'static str> {
        Self::new_parameterized(body, indices, Default::default())
    }
    pub fn new_parameterized(
        body: &[SceneVertex],
        indices: &[u32],
        parameters: crate::body_parameters::BodyParameters,
    ) -> Result<Self, &'static str> {
        parameters.validate()?;
        let morph = |p: [f64; 3]| {
            if parameters == Default::default() {
                p
            } else {
                parameters.transform(p.map(|x| x as f32)).map(f64::from)
            }
        };

        let mut normals = vec![Vec3::ZERO; body.len()];
        for t in indices.chunks_exact(3) {
            let [a, b, c] = [t[0] as usize, t[1] as usize, t[2] as usize];
            let normal = (Vec3::from_array(body[b].position) - Vec3::from_array(body[a].position))
                .cross(Vec3::from_array(body[c].position) - Vec3::from_array(body[a].position));
            for i in [a, b, c] {
                normals[i] += normal;
            }
        }
        let mut scalp = BTreeMap::new();
        for (i, v) in body.iter().enumerate() {
            let p = Vec3::from_array(v.position);
            if p.y > 0.735
                && (p.z < 0.095 || p.y > 0.775 || p.x.abs() > 0.075)
                && normals[i]
                    .normalize_or_zero()
                    .dot((p - Vec3::new(0.0, 0.72, 0.055)).normalize_or_zero())
                    > 0.5
            {
                let cell = p.to_array().map(|x| (x / 0.010).floor() as i32);
                scalp.entry(cell).or_insert((
                    p + normals[i].normalize_or_zero() * 0.001,
                    (i, normals[i].normalize_or_zero() * 0.001),
                ));
            }
        }
        let (mut roots, mut root_bindings): (Vec<_>, Vec<_>) = scalp.into_values().unzip();
        let positions: Vec<_> = body
            .iter()
            .map(|v| morph(v.position.map(f64::from)))
            .collect();
        let triangles: Vec<_> = indices
            .chunks_exact(3)
            .map(|t| [t[0] as usize, t[1] as usize, t[2] as usize])
            .filter(|t| t.iter().any(|i| body[*i].position[1] > 0.30))
            .collect();
        let collider = TriangleMesh::new(&positions, &triangles)?;
        let mut rods = Vec::with_capacity(roots.len());
        for (i, root) in roots.iter().enumerate() {
            // A short front fringe stays above the eyes after gravitational
            // relaxation; long crown guides cannot hold a swept authoring curve
            // without product, braiding or additional styling constraints.
            let front_fringe = root.z > 0.03 && root.x.abs() < 0.08;
            let length = if front_fringe {
                0.035 + (root.y - 0.755).max(0.0) * 0.4
            } else {
                0.25 + (i % 11) as f32 * 0.004
            };
            // Start from a collision-free scalp-following groom, then let the rod relax.
            // The ellipsoid is used only for authoring the initial curve, never as a collider.
            let center = Vec3::new(0.0, 0.71, 0.06);
            let radii = Vec3::new(0.112, 0.12, 0.125);
            let local = (*root - center) / radii;
            let polar = (local.y / local.length()).clamp(-1.0, 1.0).acos();
            let azimuth = if local.x.abs() + local.z.abs() < 0.05 {
                -std::f32::consts::FRAC_PI_2
            } else {
                local.z.atan2(local.x)
            };
            let curve: Vec<_> = (0..=120)
                .map(|j| {
                    let distance = length * j as f32 / 120.0;
                    let angle = (polar + distance / 0.12).min(1.65);
                    let hanging = (distance - (1.65 - polar) * 0.12).max(0.0);
                    // Sweep front roots toward the sides and back, keeping the face clear.
                    let comb = (distance / 0.075).clamp(0.0, 1.0);
                    let comb = comb * comb * (3.0 - 2.0 * comb);
                    let target = if local.x >= 0.0 {
                        -0.6
                    } else {
                        -std::f32::consts::PI + 0.6
                    };
                    let turn = (target - azimuth + std::f32::consts::PI)
                        .rem_euclid(std::f32::consts::TAU)
                        - std::f32::consts::PI;
                    let azimuth = azimuth + if local.z > 0.0 { turn * comb } else { 0.0 };
                    let envelope = center
                        + Vec3::new(
                            radii.x * angle.sin() * azimuth.cos(),
                            radii.y * angle.cos() - hanging,
                            radii.z * angle.sin() * azimuth.sin(),
                        );
                    let transition = (distance / 0.012).min(1.0);
                    root.lerp(envelope, transition)
                })
                .collect();
            let mut arc = vec![0.0f32];
            for pair in curve.windows(2) {
                arc.push(arc.last().unwrap() + pair[0].distance(pair[1]));
            }
            // The scalp-to-envelope transition can add arc length. Truncate
            // the authored path to the requested length before resampling it.
            let total = arc.last().copied().unwrap().min(length);
            let mut points: Vec<[f64; 3]> = (0..=20)
                .map(|j| {
                    let distance = total * j as f32 / 20.0;
                    let k = arc
                        .partition_point(|a| *a < distance)
                        .clamp(1, arc.len() - 1);
                    let t = (distance - arc[k - 1]) / (arc[k] - arc[k - 1]);
                    curve[k - 1].lerp(curve[k], t).to_array().map(f64::from)
                })
                .collect();
            for point in &mut points {
                *point = morph(*point);
            }
            // Keep the authored rest curve outside the exact anatomical collider.
            // Otherwise contact corrections fight the groom's own rest energy.
            for point in points.iter_mut().skip(1) {
                let (surface, normal) = collider.closest_surface(*point)?;
                let delta: [f64; 3] = std::array::from_fn(|a| point[a] - surface[a]);
                let signed: f64 = (0..3).map(|a| delta[a] * normal[a]).sum();
                let clearance = 0.001;
                if signed < clearance {
                    for axis in 0..3 {
                        point[axis] += normal[axis] * (clearance - signed);
                    }
                }
            }
            rods.push(HairRod::new(points, HairMaterial::default())?);
        }
        for (root, (vertex, offset)) in roots.iter_mut().zip(&mut root_bindings) {
            let source = Vec3::from_array(body[*vertex].position);
            let mapped_source =
                Vec3::from_array(morph(source.to_array().map(f64::from)).map(|x| x as f32));
            *root = Vec3::from_array(morph(root.to_array().map(f64::from)).map(|x| x as f32));
            *offset = *root - mapped_source;
        }
        let mut system = HairSystem::new(rods)?;
        system.iterations = 6;
        system.substeps = 2;
        system.workers = std::thread::available_parallelism().map_or(1, |n| n.get().min(8));
        let rest_roots: Vec<_> = roots
            .iter()
            .map(|p| RootPose {
                position: p.to_array().map(f64::from),
                rotation: [0.0, 0.0, 0.0, 1.0],
            })
            .collect();
        for _ in 0..2 {
            system.step(
                1.0 / 120.0,
                &rest_roots,
                [0.0; 3],
                [0.0; 3],
                std::slice::from_ref(&collider),
            )?;
        }
        // Render fibres transported by each guide frame. These add visual density,
        // not independent physical degrees of freedom. Author offsets against the
        // full anatomical surface so the initial fibres stay outside the skin.
        let surface = &collider;
        let render_offsets = system
            .rods()
            .iter()
            .map(|rod| {
                rod.positions()
                    .iter()
                    .enumerate()
                    .flat_map(|(j, p)| {
                        let q = rod.orientations()[j.min(rod.orientations().len() - 1)];
                        let frame = Quat::from_array(q.map(|x| x as f32));
                        let center = Vec3::from_array(p.map(|x| x as f32));
                        let t = j as f32 / (rod.positions().len() - 1) as f32;
                        let spread = 0.005 * (1.0 - 0.3 * t);
                        (0..FOLLOWERS).map(move |fibre| {
                            let angle = fibre as f32 * 2.399963;
                            let radius = spread * ((fibre as f32 + 0.5) / FOLLOWERS as f32).sqrt();
                            let mut point = center
                                + frame * Vec3::new(angle.cos() * radius, angle.sin() * radius, 0.);
                            let (nearest, normal) = surface
                                .closest_surface(point.to_array().map(f64::from))
                                .expect("finite rest groom");
                            let normal = Vec3::from_array(normal.map(|x| x as f32));
                            let gap =
                                (point - Vec3::from_array(nearest.map(|x| x as f32))).dot(normal);
                            if gap < 0.0002 {
                                point += normal * (0.0002 - gap);
                            }
                            frame.conjugate() * (point - center)
                        })
                    })
                    .collect()
            })
            .collect();
        println!(
            "HAIR: {} Cosserat guides, 20 segments each; E=4 GPa, fiber diameter=80 um; animated triangle contacts + guide self-contact",
            roots.len()
        );
        Ok(Self {
            system,
            targets: roots.clone(),
            #[cfg(test)]
            roots,
            root_bindings,
            collider,
            render_offsets,
        })
    }
    pub fn advance(
        &mut self,
        dt: f64,
        time: f64,
        head: Mat4,
        body: &[SceneVertex],
    ) -> Result<(), &'static str> {
        let rotation = Quat::from_mat4(&head).normalize().to_array().map(f64::from);
        if self.root_bindings.iter().any(|(i, _)| *i >= body.len()) {
            return Err("missing scalp binding vertex");
        }
        self.targets = self
            .root_bindings
            .iter()
            .map(|(i, offset)| {
                Vec3::from_array(body[*i].position) + head.transform_vector3(*offset)
            })
            .collect();
        let roots: Vec<_> = self
            .targets
            .iter()
            .map(|target| RootPose {
                position: target.to_array().map(f64::from),
                rotation,
            })
            .collect();
        let positions: Vec<_> = body.iter().map(|v| v.position.map(f64::from)).collect();
        self.collider.refit(&positions)?;
        // Keep the physical substep unchanged when the render cadence varies.
        self.system.substeps = (dt * 240.0).ceil().clamp(2.0, 32.0) as usize;
        self.system.step(
            dt,
            &roots,
            [0.0, -9.81, 0.0],
            [0.05 * (time * 1.7).sin(), 0.0, 0.02 * (time * 2.3).sin()],
            std::slice::from_ref(&self.collider),
        )
    }
    /// Static render LOD: retain every guide/follower and simplify only its
    /// polygonal sampling. Bounds both centre/section position and prelit colour.
    pub fn append_lod(
        &self,
        vertices: &mut Vec<SceneVertex>,
        indices: &mut Vec<u32>,
        tolerance: f32,
    ) {
        let mut full = Vec::new();
        let mut discarded_indices = Vec::new();
        self.append(&mut full, &mut discarded_indices);
        let stride = FOLLOWERS * 4;
        fn select(
            full: &[SceneVertex],
            stride: usize,
            a: usize,
            b: usize,
            tolerance: f32,
            out: &mut Vec<usize>,
        ) {
            let mut worst = 1_f32;
            let mut split = None;
            for j in a + 1..b {
                let t = (j - a) as f32 / (b - a) as f32;
                for k in 0..stride {
                    let left = &full[a * stride + k];
                    let right = &full[b * stride + k];
                    let sample = &full[j * stride + k];
                    let interpolated =
                        Vec3::from_array(left.position).lerp(Vec3::from_array(right.position), t);
                    let geometry =
                        (Vec3::from_array(sample.position) - interpolated).length() / tolerance;
                    let colour = (0..3)
                        .map(|c| {
                            (sample.color[c]
                                - (left.color[c] + t * (right.color[c] - left.color[c])))
                                .abs()
                                * 255.
                        })
                        .fold(0_f32, f32::max);
                    let error = geometry.max(colour);
                    if error > worst {
                        worst = error;
                        split = Some(j);
                    }
                }
            }
            if let Some(j) = split {
                select(full, stride, a, j, tolerance, out);
                select(full, stride, j, b, tolerance, out);
            } else {
                out.push(b);
            }
        }
        let mut source = 0;
        let original = full.len();
        let start = vertices.len();
        for strand in self.system.rods() {
            let count = strand.positions().len();
            let data = &full[source..source + count * stride];
            let mut selected = vec![0];
            select(data, stride, 0, count - 1, tolerance, &mut selected);
            let base = vertices.len() as u32;
            for &j in &selected {
                vertices.extend_from_slice(&data[j * stride..(j + 1) * stride]);
            }
            let stride = stride as u32;
            for j in 0..selected.len() as u32 - 1 {
                for fibre in 0..FOLLOWERS as u32 {
                    for side in 0..4 {
                        let a = base + j * stride + fibre * 4 + side;
                        let b = base + j * stride + fibre * 4 + (side + 1) % 4;
                        indices.extend([a, b, b + stride, a, b + stride, a + stride]);
                    }
                }
            }
            source += count * stride as usize;
        }
        eprintln!(
            "HAIR RENDER LOD vertices_before={original} vertices_after={} tolerance_m={tolerance} max_colour_error=1/255 guides_and_followers_preserved=true",
            vertices.len() - start
        );
    }
    pub fn append(&self, vertices: &mut Vec<SceneVertex>, indices: &mut Vec<u32>) {
        let point_count: usize = self
            .system
            .rods()
            .iter()
            .map(|rod| rod.positions().len())
            .sum();
        let segment_count = point_count - self.system.rods().len();
        vertices.reserve(point_count * FOLLOWERS * 4);
        indices.reserve(segment_count * FOLLOWERS * 24);
        let key = Vec3::new(-0.4, 0.7, 0.6).normalize();
        for (i, strand) in self.system.rods().iter().enumerate() {
            let points = strand.positions();
            let base = vertices.len() as u32;
            for (j, p) in points.iter().enumerate() {
                let p = Vec3::from_array(p.map(|x| x as f32));
                let frame = strand.orientations()[j.min(strand.orientations().len() - 1)];
                let frame = Quat::from_xyzw(
                    frame[0] as f32,
                    frame[1] as f32,
                    frame[2] as f32,
                    frame[3] as f32,
                );
                let u = frame * Vec3::X;
                let v = frame * Vec3::Y;
                let normals = [u, v, -u, -v];
                let lights = normals.map(|normal| 0.55 + 0.45 * normal.dot(key).max(0.0));
                let t = j as f32 / (points.len() - 1) as f32;
                // Render fibres represent small bundles, so distant views retain
                // continuous coverage instead of isolated subpixel fragments.
                let radius = 0.00030 * (1.0 - 0.6 * t);
                for fibre in 0..FOLLOWERS {
                    let offset = frame * self.render_offsets[i][j * FOLLOWERS + fibre];
                    let shade = 0.8 + ((i + fibre) % 7) as f32 * 0.04;
                    for (normal, light) in normals.into_iter().zip(lights) {
                        vertices.push(SceneVertex {
                            position: (p + offset + normal * radius).to_array(),
                            uv: [-7.0, 0.0],
                            color: [
                                0.12 * shade * light,
                                0.055 * shade * light,
                                0.025 * shade * light,
                                1.0,
                            ],
                        });
                    }
                }
            }
            let stride = FOLLOWERS as u32 * 4;
            // Keep each follower's adjacent rings in the post-transform cache.
            // The full triangle set and tessellation remain identical.
            for fibre in 0..FOLLOWERS as u32 {
                for j in 0..points.len() as u32 - 1 {
                    for side in 0..4 {
                        let a = base + j * stride + fibre * 4 + side;
                        let b = base + j * stride + fibre * 4 + (side + 1) % 4;
                        indices.extend([a, b, b + stride, a, b + stride, a + stride]);
                    }
                }
            }
        }
    }
    pub fn verify(&self, _head: Mat4) -> Result<(), &'static str> {
        for (rod, target) in self.system.rods().iter().zip(&self.targets) {
            let actual = Vec3::from_array(rod.positions()[0].map(|x| x as f32));
            if target.distance(actual) > 1e-5
                || rod.positions().iter().flatten().any(|x| !x.is_finite())
                || rod.max_relative_stretch() > 0.05
            {
                return Err("hair lost its scalp attachment, over-stretched or became nonfinite");
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_order_retains_every_full_resolution_triangle() {
        let body = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        ).unwrap();
        let hair = FemaleHair::new(body.mesh.vertices(), body.mesh.indices()).unwrap();
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        hair.append(&mut vertices, &mut indices);
        let mut original = Vec::<[u32; 3]>::new();
        let mut base = 0;
        let stride = FOLLOWERS as u32 * 4;
        for rod in hair.system.rods() {
            for ring in 0..rod.positions().len() as u32 - 1 {
                for fibre in 0..FOLLOWERS as u32 {
                    for side in 0..4 {
                        let a = base + ring * stride + fibre * 4 + side;
                        let b = base + ring * stride + fibre * 4 + (side + 1) % 4;
                        original.extend([[a, b, b + stride], [a, b + stride, a + stride]]);
                    }
                }
            }
            base += rod.positions().len() as u32 * stride;
        }
        assert_eq!(vertices.len(), base as usize);
        let mut reordered: Vec<[u32; 3]> = indices.chunks_exact(3)
            .map(|t| t.try_into().unwrap()).collect();
        original.sort_unstable();
        reordered.sort_unstable();
        assert_eq!(original, reordered);
    }
    #[test]
    fn authored_fringe_respects_requested_arc_length() {
        let body = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let hair = FemaleHair::new(body.mesh.vertices(), body.mesh.indices()).unwrap();
        let mut checked = 0;
        for (root, rod) in hair.roots.iter().zip(hair.system.rods()) {
            if root.z > 0.03 && root.x.abs() < 0.08 {
                let requested = 0.035 + (root.y - 0.755).max(0.0) * 0.4;
                let arc: f64 = rod
                    .positions()
                    .windows(2)
                    .map(|p| {
                        (0..3)
                            .map(|a| (p[1][a] - p[0][a]).powi(2))
                            .sum::<f64>()
                            .sqrt()
                    })
                    .sum();
                assert!(
                    arc <= f64::from(requested) + 0.002,
                    "fringe arc {arc:.4} m exceeds requested {requested:.4} m at {root:?}"
                );
                checked += 1;
            }
        }
        assert!(checked > 10);
    }

    #[test]
    fn complete_rig_loop_preserves_hair_and_scalp_bindings() {
        let body = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let rig = crate::female_rig::FemaleRig::new(body.mesh.vertices()).unwrap();
        let mut hair = FemaleHair::new(body.mesh.vertices(), body.mesh.indices()).unwrap();
        let mut maximum = 0f64;
        let started = std::time::Instant::now();
        for frame in 1..=360 {
            let time = frame as f32 / 60.;
            let head = rig.head_matrix(time);
            let mut posed = body.mesh.vertices().to_vec();
            rig.deform(&mut posed, time);
            hair.advance(1. / 60., f64::from(time), head, &posed)
                .unwrap();
            hair.verify(head).unwrap();
            maximum = maximum.max(
                hair.system
                    .rods()
                    .iter()
                    .map(HairRod::max_relative_stretch)
                    .fold(0., f64::max),
            );
            assert!(
                maximum < 0.05,
                "hair stretch {maximum} at full-loop frame {frame}"
            );
        }
        println!(
            "FULL RIG HAIR LOOP: 360 steps, max strain {maximum:.6}, {:.2} ms/step",
            started.elapsed().as_secs_f64() * 1000. / 360.
        );
    }
    #[test]
    fn resting_groom_contacts_preserve_lengths() {
        let body = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let original = FemaleHair::new(body.mesh.vertices(), body.mesh.indices()).unwrap();
        let roots: Vec<_> = original
            .roots
            .iter()
            .map(|p| RootPose {
                position: p.to_array().map(f64::from),
                rotation: [0., 0., 0., 1.],
            })
            .collect();
        for (mesh_contact, self_contact) in [(false, false), (true, false), (true, true)] {
            let mut system = original.system.clone();
            system.self_collision = self_contact;
            let started = std::time::Instant::now();
            system
                .step(
                    1. / 120.,
                    &roots,
                    [0., -9.81, 0.],
                    [0.; 3],
                    if mesh_contact {
                        std::slice::from_ref(&original.collider)
                    } else {
                        &[]
                    },
                )
                .unwrap();
            let stretch = system
                .rods()
                .iter()
                .map(HairRod::max_relative_stretch)
                .fold(0., f64::max);
            println!(
                "GROOM DIAG mesh={mesh_contact} self={self_contact}: stretch={stretch:.6}, {:.1} ms",
                started.elapsed().as_secs_f64() * 1000.
            );
            assert!(
                stretch < 0.01,
                "rest groom stretch mesh={mesh_contact}, self={self_contact}: {stretch}"
            );
        }
    }
    #[test]
    fn real_surface_contacts_and_moving_root_keep_mesh_capacity() {
        let body = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let mut hair = FemaleHair::new(body.mesh.vertices(), body.mesh.indices()).unwrap();
        assert!(hair.roots.len() > 50);
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        hair.append(&mut vertices, &mut indices);
        let sizes = (vertices.len(), indices.len());
        let start = std::time::Instant::now();
        let rig = crate::female_rig::FemaleRig::new(body.mesh.vertices()).unwrap();
        for frame in 1..=120 {
            let time = frame as f64 / 120.0;
            let head = rig.head_matrix(time as f32);
            let mut posed = body.mesh.vertices().to_vec();
            rig.deform(&mut posed, time as f32);
            hair.advance(1.0 / 120.0, time, head, &posed).unwrap();
            hair.verify(head).unwrap();
            let stretch = hair
                .system
                .rods()
                .iter()
                .map(HairRod::max_relative_stretch)
                .fold(0.0, f64::max);
            assert!(stretch < 0.05, "guide stretch at frame {frame}: {stretch}");
        }
        println!(
            "HAIR BENCH: {} guides, {:.2} ms/frame including animated BVH and contacts",
            hair.roots.len(),
            start.elapsed().as_secs_f64() * 1000.0 / 120.0
        );
        vertices.clear();
        indices.clear();
        hair.append(&mut vertices, &mut indices);
        assert_eq!((vertices.len(), indices.len()), sizes);
        voxy_render::SceneMesh::new(vertices, indices).unwrap();
    }
}
