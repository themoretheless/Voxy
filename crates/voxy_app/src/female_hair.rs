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
    skip_mesh_contacts: bool,
    #[cfg(test)]
    roots: Vec<Vec3>,
    root_bindings: Vec<(usize, Vec3)>,
    targets: Vec<Vec3>,
    collider: TriangleMesh,
    render_offsets: Vec<Vec<Vec3>>,
    render_indices: Vec<u32>,
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
        Self::new_parameterized_with_groom(body,indices,parameters,false)
    }
    /// Candidate authoring retains both follicle coordinates. Native contact
    /// qualification is required before selecting it in the normal demo.
    pub(crate) fn new_parameterized_with_groom(
        body:&[SceneVertex],indices:&[u32],parameters:crate::body_parameters::BodyParameters,
        preserve_follicle_coordinates:bool,
    )->Result<Self, &'static str> {
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
            .collect();
        let mut collider = TriangleMesh::new(&positions, &triangles)?;
        let _ = collider.enable_closed_feature_normals();
        let mut rods = Vec::with_capacity(roots.len());
        for (i, root) in roots.iter().enumerate() {
            // A short front fringe stays above the eyes after gravitational
            // relaxation; long crown guides cannot hold a swept authoring curve
            // without product, braiding or additional styling constraints.
            let front_fringe = root.z > 0.03 && root.x.abs() < 0.08;
            let length = if front_fringe {
                0.035 + (root.y - 0.755).max(0.0) * 0.4
            } else {
                // Stable, nonperiodic layering avoids eleven repeating blunt
                // tip heights without changing guide density or solver order.
                let seed = (i as u32).wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
                let word = ((seed >> ((seed >> 28) + 4)) ^ seed).wrapping_mul(277_803_737);
                let variation = ((word >> 22) ^ word) as f32 / u32::MAX as f32;
                0.24 + 0.085 * variation
            };
            // Author a scalp-following groom; actual mesh/strand contacts qualify its geometry.
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
                    // A constant target collapses entire scalp meridians into
                    // coincident hanging curves. Route each hemisphere with a
                    // monotone angular map, including its posterior roots.
                    let target = if !preserve_follicle_coordinates {
                        if local.x>=0.0 {-0.6} else {-std::f32::consts::PI+0.6}
                    } else if local.x >= 0.0 {
                        -0.85 + 0.42 * azimuth
                    } else {
                        let angle = if azimuth < 0.0 {azimuth + std::f32::consts::TAU} else {azimuth};
                        std::f32::consts::PI + 0.85 + 0.42 * (angle - std::f32::consts::PI)
                    };
                    let turn = (target - azimuth + std::f32::consts::PI)
                        .rem_euclid(std::f32::consts::TAU)
                        - std::f32::consts::PI;
                    let azimuth = azimuth + if preserve_follicle_coordinates || local.z>0.0 {turn*comb} else {0.0};
                    // The second follicle coordinate remains a radial layer:
                    // crown fibres lie outside fibres rooted further down the scalp.
                    // Equal terminal polar angle must not erase this coordinate.
                    let layer = if preserve_follicle_coordinates {0.012 * ((1.65 - polar) / 1.65).clamp(0.0, 1.0) * comb} else {0.0};
                    let envelope = center
                        + Vec3::new(
                            (radii.x + layer) * angle.sin() * azimuth.cos(),
                            radii.y * angle.cos() - hanging,
                            (radii.z + layer) * angle.sin() * azimuth.sin(),
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
        system.profiling = std::env::var_os("VOXY_HAIR_PROFILE").is_some();
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
        let mut render_indices = Vec::new();
        let mut base = 0;
        let stride = FOLLOWERS as u32 * 4;
        for strand in system.rods() {
            for fibre in 0..FOLLOWERS as u32 {
                for j in 0..strand.positions().len() as u32 - 1 {
                    for side in 0..4 {
                        let a = base + j * stride + fibre * 4 + side;
                        let b = base + j * stride + fibre * 4 + (side + 1) % 4;
                        render_indices.extend([a, b, b + stride, a, b + stride, a + stride]);
                    }
                }
            }
            base += strand.positions().len() as u32 * stride;
        }
        Ok(Self {
            #[cfg(test)]
            skip_mesh_contacts:false,
            render_indices,
            system,
            targets: roots.clone(),
            #[cfg(test)]
            roots,
            root_bindings,
            collider,
            render_offsets,
        })
    }
    #[cfg(test)]
    pub(crate) fn qualification_systems(&self, dt: f64) -> Result<Vec<physics::hair::HairLinearSystem>, &'static str> {
        self.system.rods().iter().map(|rod| rod.linear_system(dt)).collect()
    }
    pub fn advance(
        &mut self,
        dt: f64,
        time: f64,
        head: Mat4,
        body: &[SceneVertex],
    ) -> Result<(), &'static str> {
        self.advance_impl(dt,time,head,body,None)
    }
    pub(crate) fn advance_with_solver(&mut self,dt:f64,time:f64,head:Mat4,body:&[SceneVertex],solver:&mut dyn physics::hair::HairLinearSolver)->Result<(), &'static str> {
        self.advance_impl(dt,time,head,body,Some(solver))
    }
    fn advance_impl(&mut self,dt:f64,time:f64,head:Mat4,body:&[SceneVertex],solver:Option<&mut dyn physics::hair::HairLinearSolver>)->Result<(), &'static str> {
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
        self.collider.refit_with_timestep(&positions,dt)?;
        // Keep the physical substep unchanged when the render cadence varies.
        self.system.substeps = (dt * 240.0).ceil().clamp(2.0, 32.0) as usize;
        let gravity=[0.,-9.81,0.];
        let air=[0.05*(time*1.7).sin(),0.,0.02*(time*2.3).sin()];
        let meshes=std::slice::from_ref(&self.collider);
        #[cfg(test)]
        let meshes=if self.skip_mesh_contacts {&[]} else {meshes};
        if let Some(solver)=solver {
            self.system.step_with_solver(dt,&roots,gravity,air,meshes,solver)?;
        } else {self.system.step(dt,&roots,gravity,air,meshes)?;}
        if self.system.profiling {
            let profile = self.system.last_profile;
            eprintln!("HAIR PHASES structural_worker_ms={:.3} mesh_contact_worker_ms={:.3} self_contact_wall_ms={:.3}",
                profile.structural_ms, profile.mesh_contacts_ms, profile.self_contacts_ms);
        }
        Ok(())
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
    pub fn gpu_surface_frames(&self) -> Vec<voxy_render::FiberSurfaceFrame> {
        let mut frames=Vec::new();
        for rod in self.system.rods() {
            for (j,point) in rod.positions().iter().enumerate() {
                let q=Quat::from_array(rod.orientations()[j.min(rod.orientations().len()-1)].map(|v| v as f32));
                let u=q*Vec3::X;let v=q*Vec3::Y;let w=q*Vec3::Z;
                frames.push(voxy_render::FiberSurfaceFrame {
                    position_arc:[point[0] as f32,point[1] as f32,point[2] as f32,j as f32/(rod.positions().len()-1) as f32],
                    u_red:[u.x,u.y,u.z,0.055],v_green:[v.x,v.y,v.z,0.022],w_blue:[w.x,w.y,w.z,0.009],
                });
            }
        }
        frames
    }
    pub fn gpu_surface_input(&self) -> Result<voxy_render::FiberSurfaceInput,voxy_render::SceneError> {
        let frames=self.gpu_surface_frames();
        let mut offsets=Vec::new();
        for (i,rod) in self.system.rods().iter().enumerate() {
            for j in 0..rod.positions().len() {
                for fibre in 0..FOLLOWERS {
                    let o=self.render_offsets[i][j*FOLLOWERS+fibre];
                    offsets.push([o.x,o.y,o.z,0.8+((i+fibre)%7) as f32*0.04]);
                }
            }
        }
        voxy_render::FiberSurfaceInput::new(&frames,FOLLOWERS as u32,&offsets,0.00030,0.6)
    }
    pub fn append(&self, vertices: &mut Vec<SceneVertex>, indices: &mut Vec<u32>) {
        self.append_surface(vertices, indices, None);
    }
    pub fn append_with_normals(&self, vertices: &mut Vec<SceneVertex>, indices: &mut Vec<u32>, normals: &mut Vec<[f32; 3]>) {
        self.append_surface(vertices, indices, Some(normals));
    }
    fn append_surface(&self, vertices: &mut Vec<SceneVertex>, indices: &mut Vec<u32>, mut normals: Option<&mut Vec<[f32; 3]>>) {
        let point_count: usize = self
            .system
            .rods()
            .iter()
            .map(|rod| rod.positions().len())
            .sum();
        let mesh_base = vertices.len() as u32;
        vertices.reserve(point_count * FOLLOWERS * 4);
        if let Some(normals) = &mut normals { normals.reserve(point_count * FOLLOWERS * 4); }
        indices.reserve(self.render_indices.len());
        for (i, strand) in self.system.rods().iter().enumerate() {
            let points = strand.positions();
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
                let basis = glam::Mat3::from_cols(u, v, frame * Vec3::Z);
                let t = j as f32 / (points.len() - 1) as f32;
                // Render fibres represent small bundles, so distant views retain
                // continuous coverage instead of isolated subpixel fragments.
                let radius = 0.00030 * (1.0 - 0.6 * t);
                let radial_normals = [u, v, -u, -v];
                let section = radial_normals.map(|normal| normal * radius);
                for fibre in 0..FOLLOWERS {
                    let center = p + basis * self.render_offsets[i][j * FOLLOWERS + fibre];
                    let shade = 0.8 + ((i + fibre) % 7) as f32 * 0.04;
                    let color = [0.055 * shade, 0.022 * shade, 0.009 * shade, 1.0];
                    for (offset, normal) in section.into_iter().zip(radial_normals) {
                        if let Some(normals) = &mut normals { normals.push(normal.to_array()); }
                        vertices.push(SceneVertex {
                            position: (center + offset).to_array(),
                            uv: [-7.0, t],
                            color,
                        });
                    }
                }
            }
        }
        // Topology is immutable: only transport the cached local indices into
        // the caller's mesh. No guide or follower is removed.
        indices.extend(self.render_indices.iter().map(|index| mesh_base + index));
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
    fn full_resolution_basis_transport_matches_quaternion_reference() {
        let asset = voxy_render::ObjAsset::parse(include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default()).unwrap();
        let hair = FemaleHair::new(asset.mesh.vertices(), asset.mesh.indices()).unwrap();
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        hair.append(&mut vertices, &mut indices);
        let mut cursor = 0;
        for (i, strand) in hair.system.rods().iter().enumerate() {
            for (j, p) in strand.positions().iter().enumerate() {
                let p = Vec3::from_array(p.map(|x| x as f32));
                let q = strand.orientations()[j.min(strand.orientations().len()-1)];
                let frame = Quat::from_array(q.map(|x| x as f32));
                let u = frame*Vec3::X;
                let v = frame*Vec3::Y;
                let t = j as f32/(strand.positions().len()-1) as f32;
                for fibre in 0..FOLLOWERS {
                    let offset = frame*hair.render_offsets[i][j*FOLLOWERS+fibre];
                    for normal in [u,v,-u,-v] {
                        let reference = p+offset+normal*(0.00030*(1.-0.6*t));
                        assert!(Vec3::from_array(vertices[cursor].position).distance(reference)<2e-7);
                        assert_eq!(vertices[cursor].uv, [-7.,t]);
                        cursor += 1;
                    }
                }
            }
        }
        assert_eq!(cursor,vertices.len());
    }
    #[test]
    fn guide_surface_normals_preserve_full_geometry_and_frame_orientation() {
        let asset = voxy_render::ObjAsset::parse(include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default()).unwrap();
        let hair = FemaleHair::new(asset.mesh.vertices(), asset.mesh.indices()).unwrap();
        let (mut vertices, mut indices, mut normals) = (Vec::new(), Vec::new(), Vec::new());
        hair.append_with_normals(&mut vertices, &mut indices, &mut normals);
        assert_eq!(vertices.len(), normals.len());
        let mut cursor = 0;
        for rod in hair.system.rods() {
            for j in 0..rod.positions().len() {
                let frame = Quat::from_array(rod.orientations()[j.min(rod.orientations().len()-1)].map(|v| v as f32));
                for _ in 0..FOLLOWERS {
                    for expected in [frame*Vec3::X, frame*Vec3::Y, frame*(-Vec3::X), frame*(-Vec3::Y)] {
                        let n = Vec3::from_array(normals[cursor]);
                        assert!(n.distance(expected) < 1e-6);
                        assert!((n.length()-1.).abs() < 1e-6);
                        assert!(n.dot(frame*Vec3::Z).abs() < 1e-6);
                        cursor += 1;
                    }
                }
            }
        }
        let (mut reference, mut reference_indices) = (Vec::new(), Vec::new());
        hair.append(&mut reference, &mut reference_indices);
        assert_eq!(indices, reference_indices);
        assert!(vertices.iter().zip(reference).all(|(a,b)| a.position==b.position && a.uv==b.uv && a.color==b.color));
    }
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
        assert_eq!(indices, hair.render_indices);
        let mut shifted_vertices = vec![vertices[0]; 7];
        let mut shifted_indices = vec![0, 1, 2];
        hair.append(&mut shifted_vertices, &mut shifted_indices);
        assert_eq!(&shifted_indices[..3], &[0, 1, 2]);
        assert!(shifted_indices[3..].iter().zip(&indices)
            .all(|(shifted, local)| *shifted == *local + 7));
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
    #[ignore = "native GPU full-model dynamics qualification"]
    fn gpu_linear_solver_tracks_full_model_jump_with_contacts() {
        let body=voxy_render::ObjAsset::parse(include_str!("../../../assets/characters/blender-female/prepared/body-forehead-refined.obj"),voxy_render::ObjLimits::default()).unwrap();
        let preserve_follicles=std::env::var_os("VOXY_HAIR_PRESERVE_FOLLICLE_COORDINATES").is_some();
        eprintln!("HYBRID FOLLICLE PRESERVING GROOM {preserve_follicles}");
        let mut native=FemaleHair::new_parameterized_with_groom(body.mesh.vertices(),body.mesh.indices(),Default::default(),preserve_follicles).unwrap();
        let mut gpu=FemaleHair::new_parameterized_with_groom(body.mesh.vertices(),body.mesh.indices(),Default::default(),preserve_follicles).unwrap();
        if let Ok(value)=std::env::var("VOXY_HAIR_QUALIFICATION_ITERATIONS") {
            let iterations=value.parse::<usize>().unwrap();
            assert!((6..=128).contains(&iterations),"qualification iterations must be in 6..128");
            native.system.iterations=iterations;gpu.system.iterations=iterations;
        }
        eprintln!("HYBRID STRUCTURAL ITERATIONS {}",native.system.iterations);
        let joint_velocities=std::env::var_os("VOXY_HAIR_JOINT_CONTACT_VELOCITIES").is_some();
        native.system.joint_contact_velocities=joint_velocities;gpu.system.joint_contact_velocities=joint_velocities;
        eprintln!("HYBRID JOINT CONTACT VELOCITIES {joint_velocities}");
        let joint_positions=std::env::var_os("VOXY_HAIR_JOINT_CONTACT_POSITIONS").is_some();
        native.system.joint_contact_positions=joint_positions;gpu.system.joint_contact_positions=joint_positions;
        eprintln!("HYBRID JOINT CONTACT POSITIONS {joint_positions}");
        let sampled_motion=std::env::var_os("VOXY_HAIR_SAMPLE_COLLIDER_MOTION").is_some();
        native.system.sample_collider_motion=sampled_motion;gpu.system.sample_collider_motion=sampled_motion;
        eprintln!("HYBRID SAMPLED COLLIDER MOTION {sampled_motion}");
        if let Ok(value)=std::env::var("VOXY_HAIR_TERMINAL_CONTACT_ITERATIONS") {
            let iterations=value.parse::<usize>().unwrap();assert!(iterations<=32);
            native.system.terminal_contact_iterations=iterations;gpu.system.terminal_contact_iterations=iterations;
        }
        eprintln!("HYBRID TERMINAL CONTACT ITERATIONS {}",native.system.terminal_contact_iterations);
        let rig=crate::female_rig::FemaleRig::new(body.mesh.vertices()).unwrap();
        let instance=voxy_render::GraphicsOptions::default().create_instance();
        let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        eprintln!("HYBRID HAIR ADAPTER {:?}",adapter.get_info());
        let (device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let extra_refinement=std::env::var_os("VOXY_HAIR_EXTRA_DIVISION_REFINEMENT").is_some();
        eprintln!("HYBRID EXTRA DIVISION REFINEMENT {extra_refinement}");
        let extra_root=std::env::var_os("VOXY_HAIR_EXTRA_ROOT_REFINEMENT").is_some();
        eprintln!("HYBRID EXTRA ROOT REFINEMENT {extra_root}");
        let mut solver=pollster::block_on(crate::gpu_hair_solver::GpuHairLinearSolver::new_with_refinements(&device,&queue,extra_refinement,extra_root)).unwrap();
        struct NativeControl {calls:usize,perturbation:f64}
        impl physics::hair::HairLinearSolver for NativeControl {
            fn solve(&mut self,systems:&[physics::hair::HairLinearSystem])->Result<Vec<Vec<f64>>, &'static str> {
                self.calls+=1;
                systems.iter().map(|system| {
                    let mut correction=system.solve_native()?;
                    for i in system.active.clone() {correction[i]*=1.+self.perturbation;}
                    Ok(correction)
                }).collect()
            }
        }
        let contact_case=std::env::var("VOXY_HAIR_CONTACT_CASE").unwrap_or_else(|_|"full".into());
        assert!(["full","mesh","self","none"].contains(&contact_case.as_str()));
        for hair in [&mut native,&mut gpu] {
            hair.system.self_collision=matches!(contact_case.as_str(),"full"|"self");
            hair.skip_mesh_contacts=matches!(contact_case.as_str(),"none"|"self");
        }
        eprintln!("HYBRID CONTACT CASE {contact_case}");
        let cpu_control=std::env::var_os("VOXY_HAIR_CPU_CONTROL").is_some();
        let perturbation=std::env::var("VOXY_HAIR_CPU_SCALE_PERTURBATION").ok().map(|v|v.parse::<f64>().unwrap()).unwrap_or(0.);
        assert!(perturbation.is_finite() && perturbation.abs()<=1e-9);
        let mut control=NativeControl {calls:0,perturbation};
        eprintln!("HYBRID TEST BACKEND cpu_control={cpu_control} perturbation={perturbation}");
        solver.reference_audit=std::env::var_os("VOXY_HAIR_REFERENCE_AUDIT").is_some();
        let (mut position_error,mut rotation_error)=(0f64,0f64);
        let mut native_ms=0.;let mut gpu_ms=0.;
        let mut first_divergence_reported=false;
        let frames=std::env::var("VOXY_HAIR_QUALIFICATION_FRAMES").ok().map(|value|value.parse::<usize>().unwrap()).unwrap_or(30);
        assert!((30..=720).contains(&frames),"qualification frames must be in 30..720");
        let expected_calls=frames*2*(native.system.iterations+native.system.terminal_contact_iterations);
        for frame in 1..=frames {
            let time=frame as f64/120.;let bob=0.08*(std::f64::consts::TAU*time).sin().powi(2) as f32;
            let head=Mat4::from_translation(Vec3::Y*bob)*rig.head_matrix(time as f32);
            let mut posed=body.mesh.vertices().to_vec();rig.deform(&mut posed,time as f32);
            for vertex in &mut posed {vertex.position[1]+=bob;}
            let started=std::time::Instant::now();native.advance(1./120.,time,head,&posed).unwrap_or_else(|error|panic!("native frame {frame}: {error}"));native_ms+=started.elapsed().as_secs_f64()*1000.;
            let started=std::time::Instant::now();
            if cpu_control {
                gpu.advance_with_solver(1./120.,time,head,&posed,&mut control).unwrap();
            } else {gpu.advance_with_solver(1./120.,time,head,&posed,&mut solver).unwrap_or_else(|error|panic!("frame {frame}: {error}, {:?}",solver.last_error));}
            gpu_ms+=started.elapsed().as_secs_f64()*1000.;
            for (label,hair) in [("external",&gpu),("native",&native)] {
                hair.verify(head).unwrap_or_else(|error| {
                    let (rod,strain)=hair.system.rods().iter().enumerate().map(|(i,rod)|(i,rod.max_relative_stretch())).max_by(|a,b|a.1.total_cmp(&b.1)).unwrap();
                    let strand=&hair.system.rods()[rod];
                    let (segment,strain)=strand.positions().windows(2).zip(strand.rest_lengths()).enumerate().map(|(i,(points,length))| {
                        let distance=(0..3).map(|axis|(points[1][axis]-points[0][axis]).powi(2)).sum::<f64>().sqrt();
                        (i,distance/length-1.)
                    }).max_by(|a,b|a.1.abs().total_cmp(&b.1.abs())).unwrap_or((0,strain));
                    panic!("frame {frame} {label}: {error}; rod={rod} segment={segment} signed_strain={strain} endpoints={:?}",&strand.positions()[segment..=segment+1]);
                });
            }
            let mut worst=(0f64,0usize,0usize);
            for (rod,(a,b)) in native.system.rods().iter().zip(gpu.system.rods()).enumerate() {
                for (point,(a,b)) in a.positions().iter().zip(b.positions()).enumerate() {
                    let error=(0..3).map(|axis|(a[axis]-b[axis]).abs()).fold(0.,f64::max);
                    position_error=position_error.max(error);
                    if error>worst.0 {worst=(error,rod,point);}
                }
                for (a,b) in a.orientations().iter().zip(b.orientations()) {for axis in 0..4 {rotation_error=rotation_error.max((a[axis]-b[axis]).abs());}}
            }
            if worst.0>1e-6 && !first_divergence_reported {
                first_divergence_reported=true;
                eprintln!("HAIR FIRST DIVERGENCE frame={frame} rod={} point={} error_m={}",worst.1,worst.2,worst.0);
                for (label,hair) in [("native",&native),("external",&gpu)] {
                    let rod=&hair.system.rods()[worst.1];
                    for point in worst.2.saturating_sub(1)..=(worst.2+1).min(rod.positions().len()-1) {
                        let position=rod.positions()[point];
                        let (distance,normal)=hair.collider.signed_distance_closed(position).unwrap();
                        eprintln!("HAIR DIVERGENCE NEIGHBOR {label} point={point} position={position:?} body_distance_m={distance} body_normal={normal:?}");
                    }
                }
            }
            eprintln!("HYBRID HAIR FRAME frame={} max_position_error_m={} max_quaternion_component_error={}",frame,position_error,rotation_error);
        }
        eprintln!("HYBRID FULL HAIR guides={} frames={} calls={} position_error_m={} quaternion_component_error={} native_ms={} hybrid_ms={} solver_bridge_ms={}",gpu.system.rods().len(),frames,solver.calls,position_error,rotation_error,native_ms,gpu_ms,solver.elapsed_ms);
        eprintln!("HYBRID LINEAR AUDIT max_errors={:?} packing_errors={:?} stage_errors={:?} enabled={}",solver.max_linear_error,solver.max_packing_error,solver.max_stage_error,solver.reference_audit);
        assert_eq!(gpu.system.rods().len(),469);assert_eq!(if cpu_control {control.calls} else {solver.calls},expected_calls);
        assert!(position_error<1e-6,"GPU position drift {position_error}");
        assert!(rotation_error<5e-5,"GPU rotation drift {rotation_error}");
    }
    #[test]
    fn collider_preserves_lower_body_surface() {
        let body=voxy_render::ObjAsset::parse(include_str!("../../../assets/characters/blender-female/prepared/body-forehead-refined.obj"),voxy_render::ObjLimits::default()).unwrap();
        let hair=FemaleHair::new(body.mesh.vertices(),body.mesh.indices()).unwrap();
        let lowest=body.mesh.vertices().iter().min_by(|a,b|a.position[1].total_cmp(&b.position[1])).unwrap();
        let query=lowest.position.map(f64::from);
        let (closest,_)=hair.collider.closest_surface(query).unwrap();
        let distance=query.iter().zip(closest).map(|(a,b)|(a-b).powi(2)).sum::<f64>().sqrt();
        assert!(distance<1e-12,"lower-body collider surface missing: {distance} m");
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
